use esp_bootloader_esp_idf::{
  ota::OtaImageState,
  ota_updater::OtaUpdater,
  partitions::{PARTITION_TABLE_MAX_LEN, PartitionType, read_partition_table},
};

use crate::storage::with_flash;

pub const ESP_IMAGE_MAGIC: u8 = 0xE9;

/// Flash erase granularity
pub const FLASH_SECTOR_SIZE: usize = 0x1000;
/// Value of erased flash
pub const FLASH_BLANK_BYTE: u8 = 0xFF;

/// Returns (absolute_offset, size) of the slot `commit` will activate.
pub async fn target() -> Option<(u32, u32)> {
  with_flash(|flash| {
    let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
    let slot = OtaUpdater::new(flash, &mut buf)
      .ok()?
      .next_partition()
      .ok()?
      .1;
    let pt = read_partition_table(flash, &mut buf).ok()?;
    let entry = pt.find_partition(PartitionType::App(slot)).ok()??;
    Some((entry.offset(), entry.len()))
  })
  .await
}

/// Erases and programs one full sector at abs_offset.
pub async fn write_sector(abs_offset: u32, sector: &[u8; FLASH_SECTOR_SIZE]) -> bool {
  with_flash(|flash| {
    flash
      .erase(abs_offset, abs_offset + FLASH_SECTOR_SIZE as u32)
      .is_ok()
      && flash.write_nor(abs_offset, sector).is_ok()
  })
  .await
}

/// Commits the update.
pub async fn commit() -> bool {
  with_flash(|flash| {
    let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
    let mut updater = OtaUpdater::new(flash, &mut buf).ok()?;
    updater.activate_next_partition().ok()?;
    updater.set_current_ota_state(OtaImageState::New).ok()?;
    Some(())
  })
  .await
  .is_some()
}

/// Marks a freshly flashed image as working, so a rollback-enabled
/// bootloader keeps it
pub async fn confirm_boot() {
  with_flash(|flash| {
    let mut buf = [0u8; PARTITION_TABLE_MAX_LEN];
    let mut updater = OtaUpdater::new(flash, &mut buf).ok()?;
    let state = updater.current_ota_state().ok()?;
    defmt::info!("ota: boot image state={=u32:#x}", state as u32);
    if matches!(state, OtaImageState::New | OtaImageState::PendingVerify) {
      updater.set_current_ota_state(OtaImageState::Valid).ok()?;
      defmt::info!("ota: image marked valid");
    }
    Some(())
  })
  .await;
}
