#[cfg(target_arch = "xtensa")]
use core::ops::Range;

#[cfg(target_arch = "xtensa")]
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex, signal::Signal};
#[cfg(target_arch = "xtensa")]
use embedded_storage::nor_flash::{
  ErrorType as SyncErrorType, NorFlash as SyncNorFlash, ReadNorFlash as SyncReadNorFlash,
};
#[cfg(target_arch = "xtensa")]
use embedded_storage_async::nor_flash::{ErrorType, NorFlash, ReadNorFlash};
#[cfg(target_arch = "xtensa")]
use esp_storage::FlashStorage;
use sequential_storage::map::PostcardValue;
#[cfg(target_arch = "xtensa")]
use sequential_storage::{
  cache::Cache,
  map::{MapConfig, MapStorage},
};

use crate::constants::{
  DEFAULT_ANGRY_RARITY, DEFAULT_BLINK_INTERVAL, DEFAULT_DELIGHT_RARITY, DEFAULT_DIZZY_RARITY,
  DEFAULT_EYE_BLUE, DEFAULT_EYE_GREEN, DEFAULT_EYE_RED, DEFAULT_EYEROLL_RARITY,
  DEFAULT_PUPIL_HEIGHT, DEFAULT_SIN_RARITY, DEFAULT_SOUND_THRESHOLD, DEFAULT_STARTLED_RARITY,
  DEFAULT_SUSPICIOUS_RARITY, DEFAULT_WIGGLE_AMPLITUDE, DEFAULT_YAWN_RARITY,
};

// Default SSID when none is stored
const DEFAULT_SSID: &str = "Crown of the Lamb";
// Max SSID length (IEEE 802.11)
const SSID_MAX_LEN: usize = 32;
// Flash scratch buffer
#[cfg(target_arch = "xtensa")]
const FLASH_BUF_SIZE: usize = 512;
// Map keys. Postcard has no field names, so the first record can never grow.
// Anything newer gets its own key and old flash still loads
#[cfg(target_arch = "xtensa")]
const CONFIG_KEY: u8 = 0;
#[cfg(target_arch = "xtensa")]
const RARITIES_KEY: u8 = 1;

#[cfg(target_arch = "xtensa")]
pub static SAVE_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// Config partition from partitions.csv, 0x3D0000..0x3F0000
#[cfg(target_arch = "xtensa")]
const CONFIG_RANGE: Range<u32> = 0x3D_0000..0x3F_0000;

#[cfg(target_arch = "xtensa")]
static FLASH: Mutex<CriticalSectionRawMutex, Option<FlashStorage<'static>>> = Mutex::new(None);

#[cfg(target_arch = "xtensa")]
pub async fn init(flash: FlashStorage<'static>) {
  *FLASH.lock().await = Some(flash);
}

/// Async flash access for operations outside sequential-storage
#[cfg(target_arch = "xtensa")]
pub(crate) async fn with_flash<R>(op: impl FnOnce(&mut FlashStorage<'static>) -> R) -> R {
  let mut guard = FLASH.lock().await;
  op(guard.as_mut().expect("storage::init() was not called"))
}

// Wraps blocking FlashStorage as async NorFlash for sequential-storage
#[cfg(target_arch = "xtensa")]
struct BlockingFlash<'a>(&'a mut FlashStorage<'static>);

#[cfg(target_arch = "xtensa")]
impl ErrorType for BlockingFlash<'_> {
  type Error = <FlashStorage<'static> as SyncErrorType>::Error;
}

#[cfg(target_arch = "xtensa")]
impl ReadNorFlash for BlockingFlash<'_> {
  const READ_SIZE: usize = <FlashStorage<'static> as SyncReadNorFlash>::READ_SIZE;

  async fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
    SyncReadNorFlash::read(self.0, offset, bytes)
  }

  fn capacity(&self) -> usize {
    SyncReadNorFlash::capacity(self.0)
  }
}

#[cfg(target_arch = "xtensa")]
impl NorFlash for BlockingFlash<'_> {
  const WRITE_SIZE: usize = <FlashStorage<'static> as SyncNorFlash>::WRITE_SIZE;
  const ERASE_SIZE: usize = <FlashStorage<'static> as SyncNorFlash>::ERASE_SIZE;

  async fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
    SyncNorFlash::erase(self.0, from, to)
  }

  async fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
    SyncNorFlash::write(self.0, offset, bytes)
  }
}

/// Every setting the portal can change, as the firmware keeps it
pub struct StoredConfig {
  pub pupil_height: i32,
  pub wiggle_amplitude: i32,
  pub blink_interval: u32,
  pub eye_red: i32,
  pub eye_green: i32,
  pub eye_blue: i32,
  pub sound_threshold: i32,
  pub ssid: heapless::String<SSID_MAX_LEN>,
  pub rarities: Rarities,
}

/// Chance of each expression in percent, saved under RARITIES_KEY
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Rarities {
  pub sin: u32,
  pub eyeroll: u32,
  pub startled: u32,
  pub suspicious: u32,
  pub angry: u32,
  pub yawn: u32,
  pub delight: u32,
  pub dizzy: u32,
}

impl PostcardValue<'_> for Rarities {}

/// What the very first firmware wrote under CONFIG_KEY. Frozen forever,
/// one new field here and every crown out there forgets its settings
#[derive(serde::Serialize, serde::Deserialize)]
struct FirstRecord {
  pupil_height: i32,
  wiggle_amplitude: i32,
  blink_interval: u32,
  eye_red: i32,
  eye_green: i32,
  eye_blue: i32,
  sound_threshold: i32,
  sin_rarity: u32,
  eyeroll_rarity: u32,
  startled_rarity: u32,
  suspicious_rarity: u32,
  angry_rarity: u32,
  ssid: heapless::String<SSID_MAX_LEN>,
}

impl PostcardValue<'_> for FirstRecord {}

impl Default for Rarities {
  fn default() -> Self {
    Self {
      sin: DEFAULT_SIN_RARITY,
      eyeroll: DEFAULT_EYEROLL_RARITY,
      startled: DEFAULT_STARTLED_RARITY,
      suspicious: DEFAULT_SUSPICIOUS_RARITY,
      angry: DEFAULT_ANGRY_RARITY,
      yawn: DEFAULT_YAWN_RARITY,
      delight: DEFAULT_DELIGHT_RARITY,
      dizzy: DEFAULT_DIZZY_RARITY,
    }
  }
}

impl Default for StoredConfig {
  fn default() -> Self {
    Self {
      pupil_height: DEFAULT_PUPIL_HEIGHT,
      wiggle_amplitude: DEFAULT_WIGGLE_AMPLITUDE,
      blink_interval: DEFAULT_BLINK_INTERVAL,
      eye_red: DEFAULT_EYE_RED,
      eye_green: DEFAULT_EYE_GREEN,
      eye_blue: DEFAULT_EYE_BLUE,
      sound_threshold: DEFAULT_SOUND_THRESHOLD,
      ssid: DEFAULT_SSID.try_into().unwrap_or_default(),
      rarities: Rarities::default(),
    }
  }
}

impl From<&StoredConfig> for FirstRecord {
  fn from(config: &StoredConfig) -> Self {
    // The old chances still go here too, so an older firmware keeps them
    let rarities = &config.rarities;
    Self {
      pupil_height: config.pupil_height,
      wiggle_amplitude: config.wiggle_amplitude,
      blink_interval: config.blink_interval,
      eye_red: config.eye_red,
      eye_green: config.eye_green,
      eye_blue: config.eye_blue,
      sound_threshold: config.sound_threshold,
      sin_rarity: rarities.sin,
      eyeroll_rarity: rarities.eyeroll,
      startled_rarity: rarities.startled,
      suspicious_rarity: rarities.suspicious,
      angry_rarity: rarities.angry,
      ssid: config.ssid.clone(),
    }
  }
}

impl From<FirstRecord> for StoredConfig {
  fn from(record: FirstRecord) -> Self {
    Self {
      pupil_height: record.pupil_height,
      wiggle_amplitude: record.wiggle_amplitude,
      blink_interval: record.blink_interval,
      eye_red: record.eye_red,
      eye_green: record.eye_green,
      eye_blue: record.eye_blue,
      sound_threshold: record.sound_threshold,
      ssid: record.ssid,
      rarities: Rarities {
        sin: record.sin_rarity,
        eyeroll: record.eyeroll_rarity,
        startled: record.startled_rarity,
        suspicious: record.suspicious_rarity,
        angry: record.angry_rarity,
        ..Rarities::default()
      },
    }
  }
}

#[cfg(target_arch = "xtensa")]
impl StoredConfig {
  pub async fn load() -> Self {
    let mut guard = FLASH.lock().await;
    let flash = guard.as_mut().expect("storage::init() was not called");
    let mut bf = BlockingFlash(flash);
    let mut map =
      MapStorage::<u8, _, _>::new(&mut bf, MapConfig::new(CONFIG_RANGE), Cache::new_uncached());
    let mut buf = [0u8; FLASH_BUF_SIZE];
    let mut config = map
      .fetch_item::<FirstRecord>(&mut buf, &CONFIG_KEY)
      .await
      .ok()
      .flatten()
      .map_or_else(StoredConfig::default, StoredConfig::from);
    // Flash from before RARITIES_KEY keeps the old chances from the first record
    if let Some(rarities) = map
      .fetch_item::<Rarities>(&mut buf, &RARITIES_KEY)
      .await
      .ok()
      .flatten()
    {
      config.rarities = rarities;
    }
    config
  }

  pub async fn save(&self) {
    let mut guard = FLASH.lock().await;
    let flash = guard.as_mut().expect("storage::init() was not called");
    let mut bf = BlockingFlash(flash);
    let mut map =
      MapStorage::<u8, _, _>::new(&mut bf, MapConfig::new(CONFIG_RANGE), Cache::new_uncached());
    let mut buf = [0u8; FLASH_BUF_SIZE];
    map
      .store_item(&mut buf, &CONFIG_KEY, &FirstRecord::from(self))
      .await
      .ok();
    map
      .store_item(&mut buf, &RARITIES_KEY, &self.rarities)
      .await
      .ok();
  }
}

#[cfg(target_arch = "xtensa")]
unsafe extern "C" {
  fn esp_rom_software_reset_system() -> !;
}

#[cfg(target_arch = "xtensa")]
pub fn software_reset() -> ! {
  unsafe { esp_rom_software_reset_system() }
}

#[cfg(test)]
mod tests {
  use sequential_storage::map::Value;

  use super::{FirstRecord, Rarities, StoredConfig};

  // Defaults as the very first firmware wrote them. Varints, i32 zigzagged
  const FIRST_RECORD: &[u8] = &[
    0x8C, 0x01, // pupil_height 70
    0x98, 0x02, // wiggle_amplitude 140
    0xB4, 0x01, // blink_interval 180
    0x34, 0x0A, 0x00, // eye colour 26, 5, 0
    0x50, // sound_threshold 40
    0x01, 0x0F, 0x23, 0x1C, 0x37, // sin, eyeroll, startled, suspicious, angry
    17, b'C', b'r', b'o', b'w', b'n', b' ', b'o', b'f', b' ', b't', b'h', b'e', b' ', b'L', b'a',
    b'm', b'b',
  ];

  #[test]
  fn the_first_record_keeps_its_exact_bytes() {
    let mut buf = [0_u8; 64];
    let len = FirstRecord::from(&StoredConfig::default())
      .serialize_into(&mut buf)
      .unwrap();
    assert_eq!(&buf[..len], FIRST_RECORD, "old flash would stop loading");
  }

  #[test]
  fn an_old_record_keeps_its_chances_and_gets_defaults_for_new_ones() {
    let (record, _) = FirstRecord::deserialize_from(FIRST_RECORD).unwrap();
    let config = StoredConfig::from(record);
    assert_eq!(config.rarities.angry, 55);
    assert_eq!(config.ssid.as_str(), "Crown of the Lamb");
    assert_eq!(config.rarities.yawn, Rarities::default().yawn);
  }

  #[test]
  fn rarities_round_trip_on_their_own() {
    let rarities = Rarities {
      sin: 1,
      eyeroll: 2,
      startled: 3,
      suspicious: 4,
      angry: 5,
      yawn: 6,
      delight: 7,
      dizzy: 8,
    };
    let mut buf = [0_u8; 32];
    let len = rarities.serialize_into(&mut buf).unwrap();
    let (back, _) = Rarities::deserialize_from(&buf[..len]).unwrap();
    assert_eq!(
      [back.sin, back.eyeroll, back.startled, back.suspicious],
      [1, 2, 3, 4]
    );
    assert_eq!(
      [back.angry, back.yawn, back.delight, back.dizzy],
      [5, 6, 7, 8]
    );
  }
}
