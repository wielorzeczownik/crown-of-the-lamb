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
  DEFAULT_ANGRY_RARITY, DEFAULT_BLINK_INTERVAL, DEFAULT_EYE_BLUE, DEFAULT_EYE_GREEN,
  DEFAULT_EYE_RED, DEFAULT_EYEROLL_RARITY, DEFAULT_PUPIL_HEIGHT, DEFAULT_SIN_RARITY,
  DEFAULT_SOUND_THRESHOLD, DEFAULT_STARTLED_RARITY, DEFAULT_SUSPICIOUS_RARITY,
  DEFAULT_WIGGLE_AMPLITUDE,
};

// Default SSID when none is stored
const DEFAULT_SSID: &str = "Crown of the Lamb";
// Max SSID length (IEEE 802.11)
const SSID_MAX_LEN: usize = 32;
// Flash scratch buffer
#[cfg(target_arch = "xtensa")]
const FLASH_BUF_SIZE: usize = 512;
// Map key of the single stored config record
#[cfg(target_arch = "xtensa")]
const CONFIG_KEY: u8 = 0;

#[cfg(target_arch = "xtensa")]
pub static SAVE_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

// Config partition from partitions.csv: 0x3D0000..0x3F0000
#[cfg(target_arch = "xtensa")]
const CONFIG_RANGE: Range<u32> = 0x3D_0000..0x3F_0000;

#[cfg(target_arch = "xtensa")]
static FLASH: Mutex<CriticalSectionRawMutex, Option<FlashStorage<'static>>> = Mutex::new(None);

#[cfg(target_arch = "xtensa")]
pub async fn init(flash: FlashStorage<'static>) {
  *FLASH.lock().await = Some(flash);
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

#[derive(serde::Serialize, serde::Deserialize)]
pub struct StoredConfig {
  pub pupil_height: i32,
  pub wiggle_amplitude: i32,
  pub blink_interval: u32,
  pub eye_red: i32,
  pub eye_green: i32,
  pub eye_blue: i32,
  pub sound_threshold: i32,
  #[serde(default = "default_sin_rarity")]
  pub sin_rarity: u32,
  #[serde(default = "default_eyeroll_rarity")]
  pub eyeroll_rarity: u32,
  #[serde(default = "default_startled_rarity")]
  pub startled_rarity: u32,
  #[serde(default = "default_suspicious_rarity")]
  pub suspicious_rarity: u32,
  #[serde(default = "default_angry_rarity")]
  pub angry_rarity: u32,
  pub ssid: heapless::String<SSID_MAX_LEN>,
}

impl PostcardValue<'_> for StoredConfig {}

fn default_sin_rarity() -> u32 {
  DEFAULT_SIN_RARITY
}
fn default_eyeroll_rarity() -> u32 {
  DEFAULT_EYEROLL_RARITY
}
fn default_startled_rarity() -> u32 {
  DEFAULT_STARTLED_RARITY
}
fn default_suspicious_rarity() -> u32 {
  DEFAULT_SUSPICIOUS_RARITY
}
fn default_angry_rarity() -> u32 {
  DEFAULT_ANGRY_RARITY
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
      sin_rarity: DEFAULT_SIN_RARITY,
      eyeroll_rarity: DEFAULT_EYEROLL_RARITY,
      startled_rarity: DEFAULT_STARTLED_RARITY,
      suspicious_rarity: DEFAULT_SUSPICIOUS_RARITY,
      angry_rarity: DEFAULT_ANGRY_RARITY,
      ssid: DEFAULT_SSID.try_into().unwrap_or_default(),
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
    map
      .fetch_item::<StoredConfig>(&mut buf, &CONFIG_KEY)
      .await
      .ok()
      .flatten()
      .unwrap_or_default()
  }

  pub async fn save(&self) {
    let mut guard = FLASH.lock().await;
    let flash = guard.as_mut().expect("storage::init() was not called");
    let mut bf = BlockingFlash(flash);
    let mut map =
      MapStorage::<u8, _, _>::new(&mut bf, MapConfig::new(CONFIG_RANGE), Cache::new_uncached());
    let mut buf = [0u8; FLASH_BUF_SIZE];
    map.store_item(&mut buf, &CONFIG_KEY, self).await.ok();
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
