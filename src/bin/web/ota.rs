use core::sync::atomic::Ordering::Relaxed;

use cotl::ota::{self, ESP_IMAGE_MAGIC, FLASH_BLANK_BYTE, FLASH_SECTOR_SIZE};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use picoserve::{
  ResponseSent,
  io::Read,
  request::{Request, RequestBodyConnection},
  response::{IntoResponse, NoContent, ResponseWriter, StatusCode},
  routing::MethodHandlerService,
};

use super::{JSON_RESPONSE_CAP, JsonBytes};

// GET /api/version
pub async fn version() -> impl IntoResponse {
  #[derive(serde::Serialize)]
  struct Version {
    version: &'static str,
  }

  let mut data = [0u8; JSON_RESPONSE_CAP];
  let len = serde_json_core::to_slice(
    &Version {
      version: env!("CARGO_PKG_VERSION"),
    },
    &mut data,
  )
  .unwrap_or(0);

  (
    StatusCode::OK,
    ("access-control-allow-origin", "*"),
    JsonBytes { data, len },
  )
}

// POST /api/ota
pub struct OtaService;

impl MethodHandlerService for OtaService {
  async fn call_method_handler_service<R, W>(
    &self,
    _state: &(),
    _current_path_parameters: (),
    method: &str,
    mut request: Request<'_, R>,
    response_writer: W,
  ) -> Result<ResponseSent, W::Error>
  where
    R: Read,
    W: ResponseWriter<Error = R::Error>,
  {
    let status = if method == "POST" {
      upload(&mut request.body_connection).await
    } else {
      StatusCode::METHOD_NOT_ALLOWED
    };

    let sent = (status, NoContent)
      .write_to(request.body_connection.finalize().await?, response_writer)
      .await;

    // Only after the response is out, so the reset can't cut it off
    if status == StatusCode::OK {
      defmt::info!("ota: success, scheduling restart");
      cotl::control::RESTART_REQUESTED.store(true, Relaxed);
    }

    sent
  }
}

/// Fixed part of the upload deadline
const OTA_TIMEOUT_BASE_SECS: u64 = 30;
/// Slowest upload rate still accepted
const OTA_MIN_BYTES_PER_SEC: u64 = 2048;

/// Sector staging buffer, also the lock that allows only one upload at a time
static SECTOR: Mutex<CriticalSectionRawMutex, [u8; FLASH_SECTOR_SIZE]> =
  Mutex::new([FLASH_BLANK_BYTE; FLASH_SECTOR_SIZE]);

async fn upload<R: Read>(connection: &mut RequestBodyConnection<'_, R>) -> StatusCode {
  let Ok(mut sector) = SECTOR.try_lock() else {
    return StatusCode::CONFLICT;
  };
  let Some((base_offset, max_size)) = ota::target().await else {
    return StatusCode::INTERNAL_SERVER_ERROR;
  };
  let total = connection.content_length();
  if total == 0 || total > max_size as usize {
    return StatusCode::BAD_REQUEST;
  }

  // The deadline scales with the image size
  let timeout = Duration::from_secs(OTA_TIMEOUT_BASE_SECS + total as u64 / OTA_MIN_BYTES_PER_SEC);
  let mut reader = connection.body().reader().with_different_timeout(timeout);

  if stream_firmware(&mut reader, &mut sector, base_offset, total).await && ota::commit().await {
    StatusCode::OK
  } else {
    defmt::error!("ota: failed");
    StatusCode::INTERNAL_SERVER_ERROR
  }
}

async fn stream_firmware<R: Read>(
  reader: &mut R,
  sector: &mut [u8; FLASH_SECTOR_SIZE],
  base_offset: u32,
  total: usize,
) -> bool {
  defmt::info!("ota: start bytes={} offset={=u32:#x}", total, base_offset);

  let mut rx: usize = 0; // bytes received and written so far
  while rx < total {
    let want = (total - rx).min(FLASH_SECTOR_SIZE);
    sector.fill(FLASH_BLANK_BYTE); // tail of the last sector stays blank as padding
    if reader.read_exact(&mut sector[..want]).await.is_err() {
      defmt::warn!("ota: connection lost at {} of {} bytes", rx, total);
      return false;
    }
    if rx == 0 && sector[0] != ESP_IMAGE_MAGIC {
      defmt::warn!("ota: bad image magic {=u8:#x}", sector[0]);
      return false;
    }
    let abs = base_offset + rx as u32;
    if !ota::write_sector(abs, sector).await {
      defmt::error!("ota: flash write failed offset={=u32:#x}", abs);
      return false;
    }
    rx += want;
  }
  true
}
