use core::sync::atomic::Ordering::Relaxed;

use picoserve::{
  extract::Form,
  response::{IntoResponse, NoContent, StatusCode},
};

use super::{JSON_RESPONSE_CAP, JsonBytes};

// Config value ranges
const PUPIL_HEIGHT_MIN: i32 = 20;
const PUPIL_HEIGHT_MAX: i32 = 120;
const WIGGLE_AMP_MAX: i32 = 400;
const BLINK_INTERVAL_CFG_MIN: i32 = 60;
const BLINK_INTERVAL_CFG_MAX: i32 = 600;
const EYE_RED_MAX: i32 = 31; // Rgb565 5-bit
const EYE_GREEN_MAX: i32 = 63; // Rgb565 6-bit
const EYE_BLUE_MAX: i32 = 31; // Rgb565 5-bit
const SOUND_THRESHOLD_MIN: i32 = 10;
const SOUND_THRESHOLD_MAX: i32 = 400;

#[derive(serde::Deserialize)]
pub struct ConfigQuery {
  pub pupil_length: Option<i32>,
  pub wiggle_amp: Option<i32>,
  pub blink_interval: Option<i32>,
  pub eye_red: Option<i32>,
  pub eye_green: Option<i32>,
  pub eye_blue: Option<i32>,
  pub sound_threshold: Option<i32>,
  pub sin_rarity: Option<i32>,
  pub eyeroll_rarity: Option<i32>,
  pub startled_rarity: Option<i32>,
  pub suspicious_rarity: Option<i32>,
  pub angry_rarity: Option<i32>,
}

pub async fn get() -> impl IntoResponse {
  #[derive(serde::Serialize)]
  struct CurrentConfig {
    pupil_length: i32,
    wiggle_amp: i32,
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
  }

  use cotl::config::{
    ANGRY_RARITY_CFG, BLINK_INTERVAL_CFG, EYE_COLOR_BLUE, EYE_COLOR_GREEN, EYE_COLOR_RED,
    EYEROLL_RARITY_CFG, PUPIL_HEIGHT_CFG, SIN_RARITY_CFG, SOUND_THRESHOLD, STARTLED_RARITY_CFG,
    SUSPICIOUS_RARITY_CFG, WIGGLE_AMPLITUDE_CFG,
  };

  let current = CurrentConfig {
    pupil_length: PUPIL_HEIGHT_CFG.load(Relaxed),
    wiggle_amp: WIGGLE_AMPLITUDE_CFG.load(Relaxed),
    blink_interval: BLINK_INTERVAL_CFG.load(Relaxed),
    eye_red: EYE_COLOR_RED.load(Relaxed),
    eye_green: EYE_COLOR_GREEN.load(Relaxed),
    eye_blue: EYE_COLOR_BLUE.load(Relaxed),
    sound_threshold: SOUND_THRESHOLD.load(Relaxed),
    sin_rarity: SIN_RARITY_CFG.load(Relaxed),
    eyeroll_rarity: EYEROLL_RARITY_CFG.load(Relaxed),
    startled_rarity: STARTLED_RARITY_CFG.load(Relaxed),
    suspicious_rarity: SUSPICIOUS_RARITY_CFG.load(Relaxed),
    angry_rarity: ANGRY_RARITY_CFG.load(Relaxed),
  };

  let mut data = [0u8; JSON_RESPONSE_CAP];
  let len = serde_json_core::to_slice(&current, &mut data).unwrap_or(0);

  (
    StatusCode::OK,
    ("access-control-allow-origin", "*"),
    JsonBytes { data, len },
  )
}

pub async fn set(Form(query): Form<ConfigQuery>) -> impl IntoResponse {
  use cotl::config::{
    ANGRY_RARITY_CFG, BLINK_INTERVAL_CFG, EYE_COLOR_BLUE, EYE_COLOR_GREEN, EYE_COLOR_RED,
    EYEROLL_RARITY_CFG, PUPIL_HEIGHT_CFG, SIN_RARITY_CFG, SOUND_THRESHOLD, STARTLED_RARITY_CFG,
    SUSPICIOUS_RARITY_CFG, WIGGLE_AMPLITUDE_CFG,
  };

  if let Some(value) = query.pupil_length {
    PUPIL_HEIGHT_CFG.store(value.clamp(PUPIL_HEIGHT_MIN, PUPIL_HEIGHT_MAX), Relaxed);
  }
  if let Some(value) = query.wiggle_amp {
    WIGGLE_AMPLITUDE_CFG.store(value.clamp(0, WIGGLE_AMP_MAX), Relaxed);
  }
  if let Some(value) = query.blink_interval {
    BLINK_INTERVAL_CFG.store(
      value.clamp(BLINK_INTERVAL_CFG_MIN, BLINK_INTERVAL_CFG_MAX) as u32,
      Relaxed,
    );
  }
  if let Some(value) = query.eye_red {
    EYE_COLOR_RED.store(value.clamp(0, EYE_RED_MAX), Relaxed);
  }
  if let Some(value) = query.eye_green {
    EYE_COLOR_GREEN.store(value.clamp(0, EYE_GREEN_MAX), Relaxed);
  }
  if let Some(value) = query.eye_blue {
    EYE_COLOR_BLUE.store(value.clamp(0, EYE_BLUE_MAX), Relaxed);
  }
  if let Some(value) = query.sound_threshold {
    SOUND_THRESHOLD.store(
      value.clamp(SOUND_THRESHOLD_MIN, SOUND_THRESHOLD_MAX),
      Relaxed,
    );
  }
  if let Some(value) = query.sin_rarity {
    SIN_RARITY_CFG.store(value.clamp(0, 100) as u32, Relaxed);
  }
  if let Some(value) = query.eyeroll_rarity {
    EYEROLL_RARITY_CFG.store(value.clamp(0, 100) as u32, Relaxed);
  }
  if let Some(value) = query.startled_rarity {
    STARTLED_RARITY_CFG.store(value.clamp(0, 100) as u32, Relaxed);
  }
  if let Some(value) = query.suspicious_rarity {
    SUSPICIOUS_RARITY_CFG.store(value.clamp(0, 100) as u32, Relaxed);
  }
  if let Some(value) = query.angry_rarity {
    ANGRY_RARITY_CFG.store(value.clamp(0, 100) as u32, Relaxed);
  }

  cotl::storage::SAVE_SIGNAL.signal(());

  (StatusCode::NO_CONTENT, NoContent)
}

// Resets only the visual eye params
pub async fn reset() -> impl IntoResponse {
  use cotl::config::{
    BLINK_INTERVAL_CFG, EYE_COLOR_BLUE, EYE_COLOR_GREEN, EYE_COLOR_RED, PUPIL_HEIGHT_CFG,
    WIGGLE_AMPLITUDE_CFG,
  };

  let defaults = cotl::storage::StoredConfig::default();
  PUPIL_HEIGHT_CFG.store(defaults.pupil_height, Relaxed);
  WIGGLE_AMPLITUDE_CFG.store(defaults.wiggle_amplitude, Relaxed);
  BLINK_INTERVAL_CFG.store(defaults.blink_interval, Relaxed);
  EYE_COLOR_RED.store(defaults.eye_red, Relaxed);
  EYE_COLOR_GREEN.store(defaults.eye_green, Relaxed);
  EYE_COLOR_BLUE.store(defaults.eye_blue, Relaxed);

  cotl::storage::SAVE_SIGNAL.signal(());

  (StatusCode::NO_CONTENT, NoContent)
}
