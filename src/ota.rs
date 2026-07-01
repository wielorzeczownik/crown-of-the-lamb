use embedded_storage::nor_flash::NorFlash;
use esp_bootloader_esp_idf::{
  ota_updater::OtaUpdater,
  partitions::{AppPartitionSubType, PARTITION_TABLE_MAX_LEN, PartitionType, read_partition_table},
};

use crate::storage::with_flash;

pub const ESP_IMAGE_MAGIC: u8 = 0xE9;

/// Flash erase granularity: a sector must be erased before it can be written
pub const FLASH_SECTOR_SIZE: u32 = 0x1000;
/// Flash program granularity: writes happen one 4-byte word at a time
pub const FLASH_WORD_SIZE: usize = 4;
/// Value of erased flash; used to pad a final partial word
pub const FLASH_BLANK_BYTE: u8 = 0xFF;

/// Returns (absolute_offset, max_size) of the OTA slot that should receive
pub async fn ota_target() -> Option<(u32, u32)> {
  with_flash(|flash| {
    let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
    let pt = read_partition_table(flash, &mut buf).ok()?;

    let next_type = match pt.booted_partition().ok()? {
      Some(booted) if booted.partition_type() == PartitionType::App(AppPartitionSubType::Ota0) => {
        PartitionType::App(AppPartitionSubType::Ota1)
      }
      _ => PartitionType::App(AppPartitionSubType::Ota0),
    };

    let entry = pt.find_partition(next_type).ok()??;
    Some((entry.offset(), entry.len()))
  })
  .await
}

/// Erases the sector that contains abs_offset
pub async fn ota_erase_sector(abs_offset: u32) -> bool {
  with_flash(|flash| {
    flash
      .erase(abs_offset, abs_offset + FLASH_SECTOR_SIZE)
      .is_ok()
  })
  .await
}

/// Writes one flash word at abs_offset
pub async fn ota_write_word(abs_offset: u32, word: &[u8; FLASH_WORD_SIZE]) -> bool {
  with_flash(|flash| flash.write(abs_offset, word).is_ok()).await
}

/// Commits the update: rewrites otadata so the bootloader boots the new slot.
pub async fn ota_commit() -> bool {
  with_flash(|flash| {
    let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
    let mut updater = OtaUpdater::new(flash, &mut buf).ok()?;
    updater.activate_next_partition().ok()?;
    Some(())
  })
  .await
  .is_some()
}
