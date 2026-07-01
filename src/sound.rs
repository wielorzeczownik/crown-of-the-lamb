use microfft::real::rfft_64;
use micromath::F32Ext;
use oorandom::Rand32;

use crate::constants::FFT_BUFFER_SIZE;

#[derive(Clone, Copy, PartialEq, defmt::Format)]
pub enum Direction {
  Silence,
  Center,
  Left,
  Right,
}

// How many windows to hold the detected direction before allowing it to change
const DIRECTION_HOLD: u32 = 20;
// Minimum % difference between mics to recognise a direction
const DIRECTION_DIFF_PCT: i32 = 20;
// Startup debounce: keep the gate closed for the first frames
const STARTUP_GATE_FRAMES: u32 = 300;
// Usable FFT bins: rfft_64 yields N/2 complex bins; bin 0 is DC and skipped
const FFT_BINS: usize = FFT_BUFFER_SIZE / 2;
// Percentages run 0..PERCENT_SCALE
const PERCENT_SCALE: i32 = 100;

// 1-in-N odds for the randomised reactions (lower = more often)
const SKIP_REACTION_ODDS: u32 = 10; // chance to skip reacting at all
const FULL_SPIN_ODDS: u32 = 10; // chance a strong hit spins a full turn, not half
const SQUINT_ODDS: u32 = 2; // chance to squint during a rotation gesture

// Intensity thresholds (clamped percentage) selecting the rotation gesture size
const INTENSITY_QUARTER_MAX: u16 = 20; // below -> quarter turn
const INTENSITY_HALF_MAX: u16 = 50; // below -> half turn

// Rotation gesture sizes, in degrees
const ROT_QUARTER: u16 = 90;
const ROT_HALF: u16 = 180;
const ROT_FULL: u16 = 360;

// Horizontal eye offset (0..1) when looking to a side
const EYE_X_SIDE: f32 = 0.8;

// Gesture hold times by rotation size, in ms
const HOLD_QUARTER_MS: u32 = 300;
const HOLD_HALF_MS: u32 = 500;
const HOLD_FULL_MS: u32 = 800;

pub struct SoundAnalyzer {
  gate: u32,
}

impl Default for SoundAnalyzer {
  fn default() -> Self {
    Self::new()
  }
}

impl SoundAnalyzer {
  pub fn new() -> Self {
    // Start with a closed gate as a startup debounce
    Self {
      gate: STARTUP_GATE_FRAMES,
    }
  }

  pub fn is_gate_open(&self) -> bool {
    self.gate == 0
  }

  pub fn set_gate(&mut self, frames: u32) {
    self.gate = frames;
  }

  pub fn update(&mut self) {
    if self.gate > 0 {
      self.gate -= 1;
    }
  }

  /// Returns the peak magnitude in the FFT spectrum
  fn fft_max_magnitude(samples: &[f32; FFT_BUFFER_SIZE]) -> f32 {
    let mut input = *samples;
    let _ = rfft_64(&mut input);

    let mut max_magnitude: f32 = 0.0;
    for bin_idx in 1..FFT_BINS {
      let real_part = input[2 * bin_idx];
      let imag_part = input[2 * bin_idx + 1];
      let magnitude = (real_part * real_part + imag_part * imag_part).sqrt();
      if magnitude > max_magnitude {
        max_magnitude = magnitude;
      }
    }
    max_magnitude
  }

  /// Dominant direction and confidence (0-100) from both mic FFT spectra
  pub fn fft_direction_analysis(
    left_samples: &[f32; FFT_BUFFER_SIZE],
    right_samples: &[f32; FFT_BUFFER_SIZE],
  ) -> (Direction, i32) {
    let left_magnitude = Self::fft_max_magnitude(left_samples);
    let right_magnitude = Self::fft_max_magnitude(right_samples);

    let loudest = left_magnitude.max(right_magnitude);
    let diff_pct = if loudest > 0.0 {
      ((left_magnitude - right_magnitude).abs() * PERCENT_SCALE as f32 / loudest) as i32
    } else {
      0
    };

    let direction = if diff_pct < DIRECTION_DIFF_PCT {
      Direction::Center
    } else if left_magnitude > right_magnitude {
      Direction::Left
    } else {
      Direction::Right
    };

    (direction, diff_pct)
  }
}

pub struct SoundState {
  pub direction: Direction,
  rotation_degrees: u16,
  pub squint: bool,
  pub dir_changed: bool,
  hold_frames: u32,
  rng: Rand32,
}

impl SoundState {
  pub fn new(seed: u64) -> Self {
    Self {
      direction: Direction::Silence,
      rotation_degrees: 0,
      squint: false,
      dir_changed: false,
      hold_frames: 0,
      rng: Rand32::new(seed),
    }
  }

  pub fn update_from_fft(&mut self, fft_direction: Direction, diff_pct: u16) {
    // Hold timer: don't change direction until the previous reaction has played out
    if self.hold_frames > 0 {
      self.hold_frames -= 1;
      return;
    }

    // Occasionally skip, makes behaviour feel more natural and irregular
    if self.rng.rand_u32().is_multiple_of(SKIP_REACTION_ODDS) {
      return;
    }

    if fft_direction != self.direction {
      self.direction = fft_direction;
      self.hold_frames = DIRECTION_HOLD;
      self.dir_changed = true;

      let intensity = diff_pct.min(PERCENT_SCALE as u16);
      self.rotation_degrees = if intensity < INTENSITY_QUARTER_MAX {
        ROT_QUARTER
      } else if intensity < INTENSITY_HALF_MAX {
        ROT_HALF
      } else if self.rng.rand_u32().is_multiple_of(FULL_SPIN_ODDS) {
        ROT_FULL
      } else {
        ROT_HALF
      };

      self.squint = self.rotation_degrees > 0 && self.rng.rand_u32().is_multiple_of(SQUINT_ODDS);
    }
  }

  /// Target eye X position (-1..1) for the current direction
  pub fn target_eye_x(&self) -> f32 {
    match self.direction {
      Direction::Left => -EYE_X_SIDE,
      Direction::Right => EYE_X_SIDE,
      Direction::Center | Direction::Silence => 0.0,
    }
  }

  /// Hold time for the current eye gesture, in ms
  pub fn rotation_hold_ms(&self) -> u32 {
    match self.rotation_degrees {
      0 => 0,
      degrees if degrees <= ROT_QUARTER => HOLD_QUARTER_MS,
      degrees if degrees <= ROT_HALF => HOLD_HALF_MS,
      _ => HOLD_FULL_MS,
    }
  }
}
