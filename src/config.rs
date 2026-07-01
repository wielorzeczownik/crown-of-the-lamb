use core::sync::atomic::{AtomicI32, AtomicU32, Ordering};

use crate::{
  constants::{
    DEFAULT_ANGRY_RARITY, DEFAULT_BLINK_INTERVAL, DEFAULT_EYE_BLUE, DEFAULT_EYE_GREEN,
    DEFAULT_EYE_RED, DEFAULT_EYEROLL_RARITY, DEFAULT_PUPIL_HEIGHT, DEFAULT_SIN_RARITY,
    DEFAULT_SOUND_THRESHOLD, DEFAULT_STARTLED_RARITY, DEFAULT_SUSPICIOUS_RARITY,
    DEFAULT_WIGGLE_AMPLITUDE,
  },
  storage::StoredConfig,
};

macro_rules! config_params {
  ($(
    $atomic:ident: $atomic_ty:ty = $default:expr,
    stored $field:ident,
    fn $getter:ident -> $get_ty:ty = $convert:expr;
  )*) => {
    $(
      pub static $atomic: $atomic_ty = <$atomic_ty>::new($default);

      pub fn $getter() -> $get_ty {
        let convert = $convert;
        convert($atomic.load(Ordering::Relaxed))
      }
    )*

    /// Loads every persisted parameter
    pub fn apply_stored(stored: &StoredConfig) {
      $( $atomic.store(stored.$field, Ordering::Relaxed); )*
    }

    /// Copies the current runtime atomics back into stored for persistence
    pub fn capture_into(stored: &mut StoredConfig) {
      $( stored.$field = $atomic.load(Ordering::Relaxed); )*
    }
  };
}

config_params! {
  // Pupil slit height (ellipse half-height)
  PUPIL_HEIGHT_CFG: AtomicI32 = DEFAULT_PUPIL_HEIGHT,
    stored pupil_height,
    fn pupil_height -> i32 = |value: i32| value;
  // Wiggle amplitude stored
  WIGGLE_AMPLITUDE_CFG: AtomicI32 = DEFAULT_WIGGLE_AMPLITUDE,
    stored wiggle_amplitude,
    fn wiggle_amplitude -> f32 = |value: i32| value as f32 / 100.0;
  // Minimum interval between blinks in frames
  BLINK_INTERVAL_CFG: AtomicU32 = DEFAULT_BLINK_INTERVAL,
    stored blink_interval,
    fn blink_interval -> u32 = |value: u32| value;
  EYE_COLOR_RED: AtomicI32 = DEFAULT_EYE_RED,
    stored eye_red,
    fn eye_color_red -> f32 = |value: i32| value as f32;
  EYE_COLOR_GREEN: AtomicI32 = DEFAULT_EYE_GREEN,
    stored eye_green,
    fn eye_color_green -> f32 = |value: i32| value as f32;
  EYE_COLOR_BLUE: AtomicI32 = DEFAULT_EYE_BLUE,
    stored eye_blue,
    fn eye_color_blue -> f32 = |value: i32| value as f32;
  // Sound detection threshold
  SOUND_THRESHOLD: AtomicI32 = DEFAULT_SOUND_THRESHOLD,
    stored sound_threshold,
    fn sound_threshold -> i32 = |value: i32| value;
  // Expression rarities
  SIN_RARITY_CFG: AtomicU32 = DEFAULT_SIN_RARITY,
    stored sin_rarity,
    fn sin_rarity -> u32 = |value: u32| value;
  EYEROLL_RARITY_CFG: AtomicU32 = DEFAULT_EYEROLL_RARITY,
    stored eyeroll_rarity,
    fn eyeroll_rarity -> u32 = |value: u32| value;
  STARTLED_RARITY_CFG: AtomicU32 = DEFAULT_STARTLED_RARITY,
    stored startled_rarity,
    fn startled_rarity -> u32 = |value: u32| value;
  SUSPICIOUS_RARITY_CFG: AtomicU32 = DEFAULT_SUSPICIOUS_RARITY,
    stored suspicious_rarity,
    fn suspicious_rarity -> u32 = |value: u32| value;
  ANGRY_RARITY_CFG: AtomicU32 = DEFAULT_ANGRY_RARITY,
    stored angry_rarity,
    fn angry_rarity -> u32 = |value: u32| value;
}
