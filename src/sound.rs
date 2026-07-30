use microfft::real::rfft_64;
// On the host (test builds) std provides these float methods inherently, so
// the trait import is unused there. The device has no std and needs it.
#[cfg_attr(not(target_arch = "xtensa"), allow(unused_imports))]
use micromath::F32Ext;
use oorandom::Rand32;

use crate::constants::FFT_BUFFER_SIZE;

#[derive(Clone, Copy, PartialEq)]
#[cfg_attr(target_arch = "xtensa", derive(defmt::Format))]
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

// Gesture hold times by rotation size, in ms. A bigger gesture has to be held
// for longer, or the eye is interrupted mid-turn; checked at compile time so a
// careless edit cannot ship it.
const HOLD_QUARTER_MS: u32 = 300;
const HOLD_HALF_MS: u32 = 500;
const HOLD_FULL_MS: u32 = 800;
const _: () = assert!(HOLD_QUARTER_MS < HOLD_HALF_MS);
const _: () = assert!(HOLD_HALF_MS < HOLD_FULL_MS);

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

#[cfg(test)]
mod tests {
  use super::{
    Direction, HOLD_FULL_MS, HOLD_HALF_MS, HOLD_QUARTER_MS, ROT_FULL, ROT_HALF, ROT_QUARTER,
    STARTUP_GATE_FRAMES, SoundAnalyzer, SoundState,
  };
  use crate::constants::FFT_BUFFER_SIZE;

  fn tone(amplitude: f32) -> [f32; FFT_BUFFER_SIZE] {
    let mut samples = [0.0; FFT_BUFFER_SIZE];
    for (index, sample) in samples.iter_mut().enumerate() {
      let turns = index as f32 / FFT_BUFFER_SIZE as f32 * 4.0;
      *sample = amplitude * (turns * core::f32::consts::TAU).sin();
    }
    samples
  }

  #[test]
  fn silence_reports_centre_and_no_confidence() {
    let quiet = [0.0; FFT_BUFFER_SIZE];
    let (direction, diff) = SoundAnalyzer::fft_direction_analysis(&quiet, &quiet);

    assert!(direction == Direction::Center);
    assert_eq!(diff, 0, "no signal cannot imply a direction");
  }

  #[test]
  fn matched_microphones_report_centre() {
    let (direction, diff) = SoundAnalyzer::fft_direction_analysis(&tone(1.0), &tone(1.0));

    assert!(direction == Direction::Center);
    assert_eq!(diff, 0);
  }

  #[test]
  fn the_louder_microphone_wins() {
    let (direction, diff) = SoundAnalyzer::fft_direction_analysis(&tone(1.0), &tone(0.2));
    assert!(direction == Direction::Left);
    assert!(diff > 0);

    let (direction, diff) = SoundAnalyzer::fft_direction_analysis(&tone(0.2), &tone(1.0));
    assert!(direction == Direction::Right);
    assert!(diff > 0);
  }

  #[test]
  fn confidence_stays_a_percentage_and_grows_with_imbalance() {
    let mut previous = -1;
    for step in 0..=10 {
      let quiet = 1.0 - step as f32 / 10.0;
      let (_, diff) = SoundAnalyzer::fft_direction_analysis(&tone(1.0), &tone(quiet));

      assert!(
        (0..=100).contains(&diff),
        "confidence {diff} left the 0..=100 range"
      );
      assert!(
        diff >= previous,
        "a wider imbalance must not lower confidence: {diff} after {previous}"
      );
      previous = diff;
    }
  }

  #[test]
  fn a_small_imbalance_still_counts_as_centre() {
    let (direction, diff) = SoundAnalyzer::fft_direction_analysis(&tone(1.0), &tone(0.95));

    assert!(diff < super::DIRECTION_DIFF_PCT);
    assert!(direction == Direction::Center);
  }

  #[test]
  fn the_gate_starts_closed_and_opens_only_after_the_startup_debounce() {
    let mut analyzer = SoundAnalyzer::new();
    assert!(
      !analyzer.is_gate_open(),
      "reacting during power-up noise would fire a bogus expression"
    );

    for _ in 0..STARTUP_GATE_FRAMES {
      assert!(!analyzer.is_gate_open());
      analyzer.update();
    }
    assert!(analyzer.is_gate_open());
  }

  #[test]
  fn the_gate_never_underflows() {
    let mut analyzer = SoundAnalyzer::new();
    analyzer.set_gate(0);
    for _ in 0..10 {
      analyzer.update();
      assert!(analyzer.is_gate_open());
    }
  }

  #[test]
  fn a_direction_change_is_held_before_the_next_one_lands() {
    let mut state = SoundState::new(7);

    // Drive until the randomised skip lets a change through.
    let mut changed = false;
    for _ in 0..200 {
      state.update_from_fft(Direction::Left, 90);
      if state.dir_changed {
        changed = true;
        break;
      }
    }
    assert!(changed, "no direction change landed in 200 windows");
    assert!(state.direction == Direction::Left);

    // The hold timer must swallow an immediate opposite reading.
    state.update_from_fft(Direction::Right, 90);
    assert!(
      state.direction == Direction::Left,
      "a reaction has to play out before the eye is allowed to whip back"
    );
  }

  #[test]
  fn rotation_size_matches_the_reported_intensity() {
    for intensity in [0_u16, 5, 19, 20, 49, 50, 100, 5_000] {
      let mut state = SoundState::new(u64::from(intensity) + 1);

      let mut landed = false;
      for _ in 0..200 {
        state.update_from_fft(Direction::Right, intensity);
        if state.dir_changed {
          landed = true;
          break;
        }
      }
      assert!(landed, "no change landed for intensity {intensity}");

      let expected: &[u16] = if intensity < 20 {
        &[ROT_QUARTER]
      } else if intensity < 50 {
        &[ROT_HALF]
      } else {
        &[ROT_HALF, ROT_FULL]
      };
      assert!(
        expected.contains(&state.rotation_degrees),
        "intensity {intensity} produced {} degrees",
        state.rotation_degrees
      );
    }
  }

  #[test]
  fn hold_time_rises_with_the_rotation_size() {
    let mut state = SoundState::new(11);

    state.rotation_degrees = 0;
    assert_eq!(state.rotation_hold_ms(), 0);
    state.rotation_degrees = ROT_QUARTER;
    assert_eq!(state.rotation_hold_ms(), HOLD_QUARTER_MS);
    state.rotation_degrees = ROT_HALF;
    assert_eq!(state.rotation_hold_ms(), HOLD_HALF_MS);
    state.rotation_degrees = ROT_FULL;
    assert_eq!(state.rotation_hold_ms(), HOLD_FULL_MS);
  }

  #[test]
  fn eye_target_stays_within_the_normalised_range() {
    for direction in [
      Direction::Silence,
      Direction::Center,
      Direction::Left,
      Direction::Right,
    ] {
      let mut state = SoundState::new(13);
      state.direction = direction;
      let target = state.target_eye_x();

      assert!(
        (-1.0..=1.0).contains(&target),
        "eye target {target} left the -1..=1 range the renderer expects"
      );
    }
  }

  #[test]
  fn left_and_right_targets_are_mirror_images() {
    let mut left = SoundState::new(17);
    left.direction = Direction::Left;
    let mut right = SoundState::new(17);
    right.direction = Direction::Right;

    assert!((left.target_eye_x() + right.target_eye_x()).abs() < f32::EPSILON);
    assert!(left.target_eye_x() < 0.0);
  }
}
