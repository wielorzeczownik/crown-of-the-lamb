//! Turns microphone input into eye reactions

use cotl::{
  config,
  constants::{FFT_BUFFER_SIZE, RARITY_SCALE, SUSPICIOUS_GAZE_OFFSET, TICK_MS},
  eye::{Expression, EyeState},
  sound::{Direction, SoundAnalyzer, SoundState},
};
use embassy_time::Instant;
use oorandom::Rand32;

// Eye-tracking lerp blend speeds
const EYE_LERP_SOUND: f32 = 0.15; // when tracking a sound gesture
const EYE_LERP_IDLE: f32 = 0.04; // when settling back to centre
const EYE_LERP_SQUINT: f32 = 0.06; // squint open/close
// Anger trigger, rapid direction changes within a frame window
const DIR_CHANGE_ANGRY_COUNT: u32 = 4;
const DIR_CHANGE_WINDOW_FRAMES: u32 = 3_000; // animation ticks
// A sound only delights a Lamb that was already nodding off
const DELIGHT_DROWSINESS: f32 = 0.3;
// Extra ms the debounce gate stays closed past the gaze gesture
const GATE_EXTRA_MS: u32 = 100;

pub struct SoundReactor {
  state: SoundState,
  analyzer: SoundAnalyzer,
  sound_eye_x: f32,
  eye_close: f32,
  gesture_start: Option<Instant>,
  gesture_duration_ms: u32,
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
      sound_eye_x: 0.0,
      eye_close: 0.0,
      gesture_start: None,
      gesture_duration_ms: 0,
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

  /// On a fresh direction change, arm the gaze gesture and maybe fire a
  /// sound-triggered expression
  fn react_to_direction_change(&mut self, eye: &mut EyeState, now: Instant) {
    if !self.state.dir_changed {
      return;
    }
    self.state.dir_changed = false;
    defmt::debug!(
      "sound: {} after_silence={}",
      self.state.direction,
      self.state.after_silence
    );

    let hold_ms = self.state.rotation_hold_ms();
    if hold_ms > 0 {
      self.gesture_start = Some(now);
      self.gesture_duration_ms = hold_ms;
      // Gate counts ticks, not ms
      self
        .analyzer
        .set_gate(((hold_ms + GATE_EXTRA_MS) as f32 / TICK_MS) as u32);
    } else {
      self.gesture_start = None;
      self.analyzer.set_gate(0);
    }

    // Sound-triggered expressions
    if eye.expr_mode == Expression::Normal {
      if eye.drowsiness() > DELIGHT_DROWSINESS && self.rolls(config::delight_rarity()) {
        // Company at last
        eye.trigger_expr(Expression::Delight, 0.0);
      } else if self.state.after_silence && self.rolls(config::startled_rarity()) {
        // A sudden noise after silence spooks the Lamb
        eye.trigger_expr(Expression::Startled, 0.0);
      } else {
        // Track rapid direction changes -> anger
        self.dir_change_count = self.dir_change_count.saturating_add(1);
        self.dir_change_window = DIR_CHANGE_WINDOW_FRAMES;
        let spun_around = self.dir_change_count >= DIR_CHANGE_ANGRY_COUNT;
        if spun_around && self.rolls(config::angry_rarity()) {
          eye.trigger_expr(Expression::Angry, 0.0);
          self.dir_change_count = 0;
        } else if spun_around && self.rolls(config::dizzy_rarity()) {
          // Too much back and forth, the room is spinning
          eye.trigger_expr(Expression::Dizzy, 0.0);
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
  }

  /// Independent rarity roll, true with probability `rarity`% (0 = never)
  fn rolls(&mut self, rarity: u32) -> bool {
    rarity > 0 && self.rng.rand_u32() % RARITY_SCALE < rarity
  }

  /// Whether a sound-driven gaze gesture is still playing out
  pub fn is_gesture_active(&self, now: Instant) -> bool {
    self.gesture_start.is_some_and(|start| {
      now.duration_since(start).as_millis() < u64::from(self.gesture_duration_ms)
    })
  }

  /// Reads one mic window while the gate is open. `read` gives one sample
  /// pair in ADC counts
  pub fn sample<F>(&mut self, mut read: F)
  where
    F: FnMut() -> (Option<f32>, Option<f32>),
  {
    if !self.analyzer.is_gate_open() {
      return;
    }
    let mut left_buffer = [0.0; FFT_BUFFER_SIZE];
    let mut right_buffer = [0.0; FFT_BUFFER_SIZE];
    let (mut last_left, mut last_right) = (0.0, 0.0);
    for (left_out, right_out) in left_buffer.iter_mut().zip(&mut right_buffer) {
      let (left, right) = read();
      last_left = left.unwrap_or(last_left);
      last_right = right.unwrap_or(last_right);
      *left_out = last_left;
      *right_out = last_right;
    }
    let (direction, diff_pct) = SoundAnalyzer::fft_direction_analysis(
      &left_buffer,
      &right_buffer,
      config::sound_threshold() as f32,
    );
    self.state.update_from_fft(direction, diff_pct as u16);
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
