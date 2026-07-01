use core::sync::atomic::Ordering::Relaxed;

use cotl::ota::{
  ESP_IMAGE_MAGIC, FLASH_BLANK_BYTE, FLASH_SECTOR_SIZE, FLASH_WORD_SIZE, ota_target,
};
use picoserve::{
  ResponseSent,
  response::{IntoResponse as _, NoContent, ResponseWriter, StatusCode},
  routing::MethodHandlerService,
};

/// Size of the per-iteration buffer used to stream the firmware off the socket
const NET_READ_CHUNK: usize = 64;

// GET /api/version
pub async fn version() -> impl picoserve::response::IntoResponse {
  #[derive(serde::Serialize)]
  struct Version {
    version: &'static str,
  }

  let mut data = [0u8; super::JSON_RESPONSE_CAP];
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
    super::JsonBytes { data, len },
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
    mut request: picoserve::request::Request<'_, R>,
    response_writer: W,
  ) -> Result<ResponseSent, W::Error>
  where
    R: picoserve::io::Read,
    W: ResponseWriter<Error = R::Error>,
  {
    if method != "POST" {
      return (StatusCode::METHOD_NOT_ALLOWED, NoContent)
        .write_to(request.body_connection.finalize().await?, response_writer)
        .await;
    }

    let total = request.body_connection.content_length();

    let Some((base_offset, max_size)) = ota_target().await else {
      return (StatusCode::INTERNAL_SERVER_ERROR, NoContent)
        .write_to(request.body_connection.finalize().await?, response_writer)
        .await;
    };

    if total == 0 || total > max_size as usize {
      return (StatusCode::BAD_REQUEST, NoContent)
        .write_to(request.body_connection.finalize().await?, response_writer)
        .await;
    }

    let ok = {
      let body = request.body_connection.body();
      let mut reader = body.reader();
      stream_firmware(&mut reader, base_offset, total).await
    };

    if ok {
      defmt::info!("ota: success, scheduling restart");
      cotl::control::RESTART_REQUESTED.store(true, Relaxed);
    } else {
      defmt::error!("ota: failed");
    }

    let status = if ok {
      StatusCode::OK
    } else {
      StatusCode::INTERNAL_SERVER_ERROR
    };

    (status, NoContent)
      .write_to(request.body_connection.finalize().await?, response_writer)
      .await
  }
}

#[allow(clippy::large_stack_frames)]
async fn stream_firmware<R: embedded_io_async::Read>(
  reader: &mut R,
  base_offset: u32,
  total: usize,
) -> bool {
  let mut read_buf = [0u8; NET_READ_CHUNK];
  let mut word = [FLASH_BLANK_BYTE; FLASH_WORD_SIZE]; // accumulator tail bytes stay blank as padding
  let mut wpos: usize = 0; // bytes filled in word (0..FLASH_WORD_SIZE)
  let mut fofs: u32 = 0; // write offset within the OTA partition
  let mut magic_ok = false;
  let mut rx: usize = 0; // total bytes received from network

  defmt::info!("ota: start bytes={} offset={=u32:#x}", total, base_offset);

  loop {
    if rx >= total {
      break;
    }
    let want = (total - rx).min(read_buf.len());
    let n = match reader.read(&mut read_buf[..want]).await {
      Ok(0) | Err(_) => break,
      Ok(n) => n,
    };

    for &byte in &read_buf[..n] {
      if !magic_ok {
        if byte != ESP_IMAGE_MAGIC {
          defmt::warn!("ota: bad image magic {=u8:#x}", byte);
          return false;
        }
        magic_ok = true;
      }
      word[wpos] = byte;
      wpos += 1;
      rx += 1;

      if wpos == FLASH_WORD_SIZE {
        let abs = base_offset + fofs;
        if fofs.is_multiple_of(FLASH_SECTOR_SIZE) && !cotl::ota::ota_erase_sector(abs).await {
          defmt::error!("ota: erase failed offset={=u32:#x}", abs);
          return false;
        }
        if !cotl::ota::ota_write_word(abs, &word).await {
          defmt::error!("ota: write failed offset={=u32:#x}", abs);
          return false;
        }
        fofs += FLASH_WORD_SIZE as u32;
        word = [FLASH_BLANK_BYTE; FLASH_WORD_SIZE];
        wpos = 0;
      }
    }
  }

  // Write remaining bytes padded with FLASH_BLANK_BYTE
  if wpos > 0 {
    let abs = base_offset + fofs;
    if fofs.is_multiple_of(FLASH_SECTOR_SIZE) && !cotl::ota::ota_erase_sector(abs).await {
      defmt::error!("ota: erase failed offset={=u32:#x}", abs);
      return false;
    }
    if !cotl::ota::ota_write_word(abs, &word).await {
      defmt::error!("ota: write failed offset={=u32:#x}", abs);
      return false;
    }
  }

  rx == total && cotl::ota::ota_commit().await
}
