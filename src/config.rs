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

#[cfg(test)]
mod tests {
  use core::sync::atomic::Ordering::Relaxed;

  use super::{
    ANGRY_RARITY_CFG, BLINK_INTERVAL_CFG, EYE_COLOR_BLUE, EYE_COLOR_GREEN, EYE_COLOR_RED,
    EYEROLL_RARITY_CFG, PUPIL_HEIGHT_CFG, SIN_RARITY_CFG, SOUND_THRESHOLD, STARTLED_RARITY_CFG,
    SUSPICIOUS_RARITY_CFG, WIGGLE_AMPLITUDE_CFG, apply_stored, capture_into,
  };
  use crate::storage::StoredConfig;

  fn distinct() -> StoredConfig {
    StoredConfig {
      pupil_height: 41,
      wiggle_amplitude: 42,
      blink_interval: 43,
      eye_red: 11,
      eye_green: 22,
      eye_blue: 13,
      sound_threshold: 44,
      sin_rarity: 45,
      eyeroll_rarity: 46,
      startled_rarity: 47,
      suspicious_rarity: 48,
      angry_rarity: 49,
      ssid: StoredConfig::default().ssid,
    }
  }

  #[test]
  fn apply_stored_then_capture_into_round_trips_every_field() {
    let source = distinct();
    apply_stored(&source);

    let mut captured = StoredConfig::default();
    capture_into(&mut captured);

    assert_eq!(captured.pupil_height, source.pupil_height);
    assert_eq!(captured.wiggle_amplitude, source.wiggle_amplitude);
    assert_eq!(captured.blink_interval, source.blink_interval);
    assert_eq!(captured.eye_red, source.eye_red);
    assert_eq!(captured.eye_green, source.eye_green);
    assert_eq!(captured.eye_blue, source.eye_blue);
    assert_eq!(captured.sound_threshold, source.sound_threshold);
    assert_eq!(captured.sin_rarity, source.sin_rarity);
    assert_eq!(captured.eyeroll_rarity, source.eyeroll_rarity);
    assert_eq!(captured.startled_rarity, source.startled_rarity);
    assert_eq!(captured.suspicious_rarity, source.suspicious_rarity);
    assert_eq!(captured.angry_rarity, source.angry_rarity);
  }

  #[test]
  fn capture_into_overwrites_rather_than_merges() {
    apply_stored(&distinct());

    let mut target = StoredConfig {
      pupil_height: -1,
      wiggle_amplitude: -1,
      blink_interval: 9_999,
      eye_red: -1,
      eye_green: -1,
      eye_blue: -1,
      sound_threshold: -1,
      sin_rarity: 9_999,
      eyeroll_rarity: 9_999,
      startled_rarity: 9_999,
      suspicious_rarity: 9_999,
      angry_rarity: 9_999,
      ssid: StoredConfig::default().ssid,
    };
    capture_into(&mut target);

    assert_ne!(target.pupil_height, -1);
    assert_ne!(target.blink_interval, 9_999);
    assert_ne!(target.angry_rarity, 9_999);
  }

  #[test]
  fn capture_into_leaves_the_ssid_alone() {
    apply_stored(&distinct());

    let mut target = StoredConfig::default();
    let ssid_before = target.ssid.clone();
    capture_into(&mut target);

    assert_eq!(target.ssid, ssid_before);
  }

  #[test]
  fn wiggle_amplitude_is_read_back_as_hundredths() {
    WIGGLE_AMPLITUDE_CFG.store(140, Relaxed);
    assert!((super::wiggle_amplitude() - 1.4).abs() < 1e-6);

    WIGGLE_AMPLITUDE_CFG.store(0, Relaxed);
    assert!(super::wiggle_amplitude().abs() < f32::EPSILON);
  }

  #[test]
  fn colour_channels_are_read_back_unscaled() {
    EYE_COLOR_RED.store(26, Relaxed);
    EYE_COLOR_GREEN.store(5, Relaxed);
    EYE_COLOR_BLUE.store(31, Relaxed);

    assert!((super::eye_color_red() - 26.0).abs() < f32::EPSILON);
    assert!((super::eye_color_green() - 5.0).abs() < f32::EPSILON);
    assert!((super::eye_color_blue() - 31.0).abs() < f32::EPSILON);
  }

  #[test]
  fn integer_parameters_are_read_back_verbatim() {
    PUPIL_HEIGHT_CFG.store(70, Relaxed);
    BLINK_INTERVAL_CFG.store(180, Relaxed);
    SOUND_THRESHOLD.store(40, Relaxed);
    SIN_RARITY_CFG.store(1, Relaxed);
    EYEROLL_RARITY_CFG.store(15, Relaxed);
    STARTLED_RARITY_CFG.store(35, Relaxed);
    SUSPICIOUS_RARITY_CFG.store(28, Relaxed);
    ANGRY_RARITY_CFG.store(55, Relaxed);

    assert_eq!(super::pupil_height(), 70);
    assert_eq!(super::blink_interval(), 180);
    assert_eq!(super::sound_threshold(), 40);
    assert_eq!(super::sin_rarity(), 1);
    assert_eq!(super::eyeroll_rarity(), 15);
    assert_eq!(super::startled_rarity(), 35);
    assert_eq!(super::suspicious_rarity(), 28);
    assert_eq!(super::angry_rarity(), 55);
  }
}
