//! Turns microphone input into eye reactions

use cotl::{
  config,
  constants::{FFT_BUFFER_SIZE, RARITY_SCALE, SUSPICIOUS_GAZE_OFFSET},
  eye::{Expression, EyeState},
  sound::{Direction, SoundAnalyzer, SoundState},
};
use embassy_time::Instant;
use oorandom::Rand32;

// ADC samples per measurement window
const ADC_SAMPLES: usize = 16;
// Eye-tracking lerp blend speeds
const EYE_LERP_SOUND: f32 = 0.15; // when tracking a sound gesture
const EYE_LERP_IDLE: f32 = 0.04; // when settling back to centre
const EYE_LERP_SQUINT: f32 = 0.06; // squint open/close
// Anger trigger: rapid direction changes within a frame window
const DIR_CHANGE_ANGRY_COUNT: u32 = 4;
const DIR_CHANGE_WINDOW_FRAMES: u32 = 3_000;
// Extra ms the debounce gate stays closed past the gaze gesture
const GATE_EXTRA_MS: u32 = 100;

pub struct SoundReactor {
  state: SoundState,
  analyzer: SoundAnalyzer,
  left_buffer: [f32; FFT_BUFFER_SIZE],
  right_buffer: [f32; FFT_BUFFER_SIZE],
  buffer_idx: usize,
  sound_eye_x: f32,
  eye_close: f32,
  gesture_start: Option<Instant>,
  gesture_duration_ms: u32,
  prev_direction: Direction,
  dir_change_count: u32,
  dir_change_window: u32,
  rng: Rand32,
}

impl SoundReactor {
  pub fn new(seed: u64) -> Self {
    let mut rng = Rand32::new(seed);
    // Derive a distinct seed for the SoundState PRNG from this one
    let state_seed = (u64::from(rng.rand_u32()) << 32) | u64::from(rng.rand_u32());
    Self {
      state: SoundState::new(state_seed),
      analyzer: SoundAnalyzer::new(),
      left_buffer: [0.0; FFT_BUFFER_SIZE],
      right_buffer: [0.0; FFT_BUFFER_SIZE],
      buffer_idx: 0,
      sound_eye_x: 0.0,
      eye_close: 0.0,
      gesture_start: None,
      gesture_duration_ms: 0,
      prev_direction: Direction::Silence,
      dir_change_count: 0,
      dir_change_window: 0,
      rng,
    }
  }

  /// Advances per-frame timers and processes a fresh direction change
  pub fn update(&mut self, eye: &mut EyeState, now: Instant) {
    self.decay_anger_window();
    self.react_to_direction_change(eye, now);
    self.analyzer.update();
  }

  /// The anger counter decays while the room stays silent
  fn decay_anger_window(&mut self) {
    if self.state.direction == Direction::Silence {
      self.dir_change_window = self.dir_change_window.saturating_sub(1);
      if self.dir_change_window == 0 {
        self.dir_change_count = 0;
      }
    }
  }

  /// On a fresh direction change: arm the gaze gesture and maybe fire a
  /// sound-triggered expression
  fn react_to_direction_change(&mut self, eye: &mut EyeState, now: Instant) {
    if !self.state.dir_changed {
      return;
    }
    self.state.dir_changed = false;

    let hold_ms = self.state.rotation_hold_ms();
    if hold_ms > 0 {
      self.gesture_start = Some(now);
      self.gesture_duration_ms = hold_ms;
      self.analyzer.set_gate(hold_ms + GATE_EXTRA_MS);
    } else {
      self.gesture_start = None;
      self.analyzer.set_gate(0);
    }

    // Sound-triggered expressions
    if eye.expr_mode == Expression::Normal {
      if self.prev_direction == Direction::Silence && self.rolls(config::startled_rarity()) {
        // Sudden sound from silence -> startle
        eye.trigger_expr(Expression::Startled, 0.0);
      } else {
        // Track rapid direction changes -> anger
        self.dir_change_count = self.dir_change_count.saturating_add(1);
        self.dir_change_window = DIR_CHANGE_WINDOW_FRAMES;
        if self.dir_change_count >= DIR_CHANGE_ANGRY_COUNT && self.rolls(config::angry_rarity()) {
          eye.trigger_expr(Expression::Angry, 0.0);
          self.dir_change_count = 0;
        } else if self.rolls(config::suspicious_rarity()) {
          // Random suspicion
          let side = if self.state.direction == Direction::Left {
            -SUSPICIOUS_GAZE_OFFSET
          } else {
            SUSPICIOUS_GAZE_OFFSET
          };
          eye.trigger_expr(Expression::Suspicious, side);
        } else if self.rolls(config::eyeroll_rarity()) {
          eye.trigger_expr(Expression::EyeRoll, 0.0);
        }
      }
    }
    self.prev_direction = self.state.direction;
  }

  /// Independent rarity roll: true with probability `rarity`% (0 = never)
  fn rolls(&mut self, rarity: u32) -> bool {
    rarity > 0 && self.rng.rand_u32() % RARITY_SCALE < rarity
  }

  /// Whether a sound-driven gaze gesture is still playing out
  pub fn is_gesture_active(&self, now: Instant) -> bool {
    self.gesture_start.is_some_and(|start| {
      now.duration_since(start).as_millis() < u64::from(self.gesture_duration_ms)
    })
  }

  /// Reads one analysis window from the microphones, but only while the
  /// debounce gate is open
  pub fn sample<F>(&mut self, mut read: F)
  where
    F: FnMut() -> (Option<f32>, Option<f32>),
  {
    if !self.analyzer.is_gate_open() {
      return;
    }
    for _ in 0..ADC_SAMPLES {
      let (left, right) = read();
      let idx = self.buffer_idx % FFT_BUFFER_SIZE;
      if let Some(left) = left {
        self.left_buffer[idx] = left;
      }
      if let Some(right) = right {
        self.right_buffer[idx] = right;
      }
      self.buffer_idx += 1;
    }
    if self.buffer_idx >= FFT_BUFFER_SIZE {
      let (direction, diff_pct) =
        SoundAnalyzer::fft_direction_analysis(&self.left_buffer, &self.right_buffer);
      self.state.update_from_fft(direction, diff_pct as u16);
      self.buffer_idx = 0;
    }
  }

  /// Smoothed horizontal gaze target driven by the current sound direction
  pub fn track_eye_x(&mut self, gesture_active: bool) -> f32 {
    let target = if gesture_active {
      self.state.target_eye_x()
    } else {
      0.0
    };
    let speed = if gesture_active {
      EYE_LERP_SOUND
    } else {
      EYE_LERP_IDLE
    };
    self.sound_eye_x += (target - self.sound_eye_x) * speed;
    self.sound_eye_x
  }

  /// Smoothed squint amount
  pub fn track_squint(&mut self, gesture_active: bool) -> f32 {
    let target = if self.state.squint && gesture_active {
      1.0
    } else {
      0.0
    };
    self.eye_close += (target - self.eye_close) * EYE_LERP_SQUINT;
    self.eye_close
  }
}
