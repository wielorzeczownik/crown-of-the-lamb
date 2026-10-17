#[cfg_attr(not(target_arch = "xtensa"), allow(unused_imports))]
use micromath::F32Ext;
use oorandom::Rand32;

use crate::constants::{RARITY_SCALE, TICK_MS};

// Gaze rides a spring, snaps over and overshoots a hair
const GAZE_STIFFNESS: f32 = 0.3;
const GAZE_DAMPING: f32 = 0.6;

// Idle saccades, normalised -1..1
const SACCADE_RANGE_X: f32 = 0.4;
const SACCADE_RANGE_Y: f32 = 0.2;
const FIXATION_MIN: u32 = 60; // ticks
const FIXATION_SPREAD: u32 = 180;
// Most of the time the eye only twitches around the same spot
const MICROSACCADE_CHANCE: f32 = 0.6;
const MICROSACCADE_RANGE: f32 = 0.03;
const MICROSACCADE_HOLD_MIN: u32 = 20;
const MICROSACCADE_HOLD_SPREAD: u32 = 40;
// Slow wander while fixating, so it never stands perfectly still
const DRIFT_AMPLITUDE: f32 = 0.012;
// Real eyes often blink on a long jump
const SACCADE_BLINK_DISTANCE: f32 = 0.3;
const SACCADE_BLINK_CHANCE: f32 = 0.35;

// Blink timing, lid progress per tick
const BLINK_CLOSE_SPEED: f32 = 0.2;
const BLINK_OPEN_SPEED: f32 = 0.11;
const BLINK_SPEED_JITTER: f32 = 0.25; // +- share of the speed, no two blinks alike
const BLINK_HOLD_MIN: u32 = 1;
const BLINK_HOLD_SPREAD: u32 = 3;
const BLINK_INTERVAL_MIN: u32 = 180;
const BLINK_INTERVAL_RANGE: u32 = 300;
const DOUBLE_BLINK_CHANCE: f32 = 0.12;
const DOUBLE_BLINK_GAP: u32 = 10; // ticks

// Pupil slowly breathes in and out
const PUPIL_BREATH: f32 = 0.03;
// Pupil opens up in the dark behind the lid, then flinches at the light
const DARK_DILATE: f32 = 0.2;
const DARK_FULL_TICKS: f32 = 60.0;
const LIGHT_REFLEX_MIN: f32 = 0.3; // even a quick blink makes it flinch a bit
const LIGHT_REFLEX_SHRINK: f32 = 0.15;
const LIGHT_REFLEX_DECAY: f32 = 0.97;

// Squash and stretch, fast moves pull the pupil long and thin
const STRETCH_GAIN: f32 = 2.5;
const STRETCH_Y_WEIGHT: f32 = 0.6; // y moves fewer pixels per unit than x
const STRETCH_MIN: f32 = 0.75;
const STRETCH_MAX: f32 = 1.4;
const STRETCH_FOLLOW: f32 = 0.5; // smooths out tremor flicker

// A quiet minute makes the Lamb sleepy, fully drowsy five minutes later
const DROWSY_AFTER_TICKS: u32 = (60_000.0 / TICK_MS) as u32;
const DROWSY_RAMP_TICKS: f32 = 5.0 * 60_000.0 / TICK_MS;
const DROWSY_FULL_TICKS: u32 = DROWSY_AFTER_TICKS + DROWSY_RAMP_TICKS as u32;
const DROWSY_SQUINT: f32 = 0.35;
const DROWSY_BLINK_HOLD: f32 = 12.0; // extra closed ticks

// Boot greeting, the eye wakes up instead of just appearing
const WAKE_CLOSED_TICKS: u32 = 30;
const WAKE_OPEN_SPEED: f32 = 0.025;
const WAKE_SQUINT: f32 = 0.6; // heavy lids that clear up on their own
const WAKE_DOUBLE_BLINK_GAP: u32 = 40;

// Idle quirks, rolled on each proper saccade while nothing is heard
const QUIRK_CHANCE: f32 = 0.18;
const GLANCE_RANGE_X: f32 = 0.75;
const GLANCE_HOLD_MIN: u32 = 18;
const GLANCE_HOLD_SPREAD: u32 = 12;
const SCAN_MIN: u8 = 3;
const SCAN_SPREAD: u32 = 2;
const SCAN_RANGE_X: f32 = 0.6;
const SCAN_HOLD_MIN: u32 = 12;
const SCAN_HOLD_SPREAD: u32 = 10;
const CURIOUS_TICKS: u32 = 90;
const CURIOUS_PUPIL: f32 = 0.25; // extra pupil scale at the peak
// Very sleepy Lamb sometimes nods off and snaps awake
const NOD_OFF_DROWSINESS: f32 = 0.6;
const NOD_OFF_CHANCE: f32 = 0.3;
const NOD_OFF_HOLD_MIN: u32 = 60;
const NOD_OFF_HOLD_SPREAD: u32 = 90;
const NOD_OFF_OPEN_SPEED: f32 = 0.3;

// expr_t step per ms, the expression ends at 1.0
const EXPR_SPEED_EYEROLL: f32 = 1.0 / 1500.0; // 1.5 s full eye-roll arc
const EXPR_SPEED_STARTLED: f32 = 1.0 / 1400.0; // 1.4 s slower tremor
const EXPR_SPEED_SUSPICIOUS: f32 = 1.0 / 3000.0; // 3 s hold + squint reveal
const EXPR_SPEED_ANGRY: f32 = 1.0 / 2000.0; // 2 s shake + squint
const EXPR_SPEED_YAWN: f32 = 1.0 / 3500.0; // 3.5 s, nobody rushes a good yawn
const EXPR_SPEED_DELIGHT: f32 = 1.0 / 1600.0; // 1.6 s, two happy hops
const EXPR_SPEED_DIZZY: f32 = 1.0 / 2200.0; // 2.2 s of spinning

// Yawn, only a sleepy Lamb yawns on its own
const YAWN_DROWSINESS: f32 = 0.3;
const YAWN_CLOSE: f32 = 0.8; // how far the lids sink
const YAWN_LOOK_DOWN: f32 = 0.35;
const YAWN_PUPIL: f32 = 0.15;
const YAWN_BLINK_DELAY: u32 = 8; // ticks, a blink to finish it off
// Delight
const DELIGHT_HOP: f32 = 0.35;
const DELIGHT_PUPIL: f32 = 0.45;
// Dizzy
const DIZZY_TURNS: f32 = 3.5;
const DIZZY_RADIUS: f32 = 0.5;
const DIZZY_SQUINT: f32 = 0.25;
const DIZZY_WOBBLE: f32 = 0.12;

#[derive(Clone, Copy, PartialEq)]
pub enum BlinkPhase {
  Open,
  Closing,
  Closed(u32),
  Opening,
}

/// Facial expressions the eye can play
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(target_arch = "xtensa", derive(defmt::Format))]
pub enum Expression {
  Normal,
  Sin,
  EyeRoll,
  Startled,
  Suspicious,
  Angry,
  Yawn,
  Delight,
  Dizzy,
}

impl Expression {
  /// Maps a web UI expression code to an expression
  pub fn from_web_code(code: i32) -> Option<Self> {
    Some(match code {
      1 => Self::Sin,
      2 => Self::EyeRoll,
      3 => Self::Normal,
      4 => Self::Startled,
      5 => Self::Suspicious,
      7 => Self::Angry,
      8 => Self::Yawn,
      9 => Self::Delight,
      10 => Self::Dizzy,
      _ => return None,
    })
  }

  /// Normal and Sin just hold, everything else is animated by expr_t
  fn is_animated(self) -> bool {
    !matches!(self, Self::Normal | Self::Sin)
  }

  /// How far expr_t advances per millisecond
  fn animation_speed(self) -> f32 {
    match self {
      Self::EyeRoll => EXPR_SPEED_EYEROLL,
      Self::Startled => EXPR_SPEED_STARTLED,
      Self::Suspicious => EXPR_SPEED_SUSPICIOUS,
      Self::Angry => EXPR_SPEED_ANGRY,
      Self::Yawn => EXPR_SPEED_YAWN,
      Self::Delight => EXPR_SPEED_DELIGHT,
      Self::Dizzy => EXPR_SPEED_DIZZY,
      Self::Normal | Self::Sin => 0.0,
    }
  }

  /// Expressions that move the eye up and down themselves
  fn owns_gaze_y(self) -> bool {
    matches!(
      self,
      Self::EyeRoll | Self::Startled | Self::Yawn | Self::Delight | Self::Dizzy
    )
  }
}

pub struct EyeState {
  pub x: f32,
  pub y: f32,
  pub phase: f32, // phase for pupil edge wiggle
  pub blink: f32, // 1.0 = fully open, 0.0 = closed
  pub blink_phase: BlinkPhase,
  pub blink_timer: u32,
  gaze_x: f32, // current fixation point
  gaze_y: f32,
  gaze_vx: f32,
  gaze_vy: f32,
  gaze_hold: u32,
  lid_t: f32,                      // 0..1 progress of the current lid stroke
  blink_speed: f32,                // lid_t step for the current stroke
  quiet_ticks: u32,                // ticks with no sound and no expression
  waking: bool,                    // still on the boot greeting
  nodding: bool,                   // this blink is a doze, snap open after it
  glance_back: Option<(f32, f32)>, // where to return after a glance
  scan_left: u8,
  curious_ticks: u32,
  double_blink: bool,        // next blink comes in a pair no matter what
  pub sound_x: f32,          // -1..1, set by sound detection
  pub extra_close: f32,      // sound squint
  pub expr_squint: f32,      // expression squint
  pub expr_pupil_scale: f32, // pupil dilation scale
  pub stretch: f32,          // above 1 is wide and short, below 1 tall and thin
  prev_x: f32,
  prev_y: f32,
  dark: f32,   // 0..1 how long the lid has been shut
  reflex: f32, // 0..1 light flinch after opening
  pub expr_mode: Expression,
  pub expr_t: f32, // 0.0..1.0 progress
  pub expr_param: f32,
  expr_squint_target: f32, // target set each frame by the active expression
  expr_pupil_target: f32,  // target pupil scale set each frame
  pending_expr: Option<(Expression, f32)>, // queued via queue_expr(), applied at next blink close
  rng: Rand32,
}

impl EyeState {
  pub fn new(seed: u64) -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      phase: 0.0,
      blink: 0.0,
      blink_phase: BlinkPhase::Closed(WAKE_CLOSED_TICKS),
      blink_timer: BLINK_INTERVAL_MIN + 80,
      gaze_x: 0.0,
      gaze_y: 0.0,
      gaze_vx: 0.0,
      gaze_vy: 0.0,
      gaze_hold: FIXATION_MIN,
      lid_t: 0.0,
      blink_speed: 0.0,
      quiet_ticks: 0,
      waking: true,
      nodding: false,
      glance_back: None,
      scan_left: 0,
      curious_ticks: 0,
      double_blink: false,
      sound_x: 0.0,
      extra_close: 0.0,
      expr_squint: WAKE_SQUINT,
      expr_pupil_scale: 1.0,
      stretch: 1.0,
      prev_x: 0.0,
      prev_y: 0.0,
      dark: 0.0,
      reflex: 0.0,
      expr_mode: Expression::Normal,
      expr_t: 0.0,
      expr_param: 0.0,
      expr_squint_target: 0.0,
      expr_pupil_target: 1.0,
      pending_expr: None,
      rng: Rand32::new(seed),
    }
  }

  /// Immediately start an expression (used by sound triggers and reset).
  /// Cancels any queued expression
  pub fn trigger_expr(&mut self, mode: Expression, param: f32) {
    self.pending_expr = None;
    self.set_mode(mode);
    self.expr_t = 0.0;
    self.expr_param = param;
  }

  /// Sets the active expression, logging every real transition so the current
  /// mimic is visible in the logs regardless of trigger source (web, sound,
  /// idle)
  fn set_mode(&mut self, mode: Expression) {
    if mode != self.expr_mode {
      #[cfg(target_arch = "xtensa")]
      defmt::info!("expr: active {}", mode);
    }
    self.expr_mode = mode;
  }

  /// Queue an expression to start at the next blink close (used by UI buttons)
  pub fn queue_expr(&mut self, mode: Expression, param: f32) {
    self.pending_expr = Some((mode, param));
    match self.blink_phase {
      BlinkPhase::Open | BlinkPhase::Opening => self.start_close(),
      _ => {} // already closing/closed
    }
  }

  /// Starts the lid down from wherever it is now
  fn start_close(&mut self) {
    // Inverse of the closing curve, so a blink cut short mid-open doesn't jump
    self.lid_t = (1.0 - self.blink).max(0.0).sqrt();
    self.blink_speed = BLINK_CLOSE_SPEED * self.jitter();
    self.blink_phase = BlinkPhase::Closing;
  }

  fn rand_signed(&mut self) -> f32 {
    self.rng.rand_float() * 2.0 - 1.0
  }

  fn jitter(&mut self) -> f32 {
    1.0 + self.rand_signed() * BLINK_SPEED_JITTER
  }

  /// 0 while something is going on, creeps to 1 over a long silence
  pub fn drowsiness(&self) -> f32 {
    (self.quiet_ticks.saturating_sub(DROWSY_AFTER_TICKS) as f32 / DROWSY_RAMP_TICKS).min(1.0)
  }

  /// Kicks off an idle quirk, only a glance moves the gaze itself
  fn start_quirk(&mut self) -> Option<u32> {
    match self.rng.rand_u32() % 3 {
      0 => {
        // Quick peek to one side, something caught its eye
        self.glance_back = Some((self.gaze_x, self.gaze_y));
        let side = if self.rng.rand_u32().is_multiple_of(2) {
          -1.0
        } else {
          1.0
        };
        self.gaze_x = side * GLANCE_RANGE_X;
        return Some(GLANCE_HOLD_MIN + self.rng.rand_u32() % GLANCE_HOLD_SPREAD);
      }
      1 => self.scan_left = SCAN_MIN + (self.rng.rand_u32() % SCAN_SPREAD) as u8,
      _ => self.curious_ticks = CURIOUS_TICKS,
    }
    None
  }

  /// Picks where to look next, a tiny twitch or a proper saccade
  fn next_fixation(&mut self, drowsy: f32, hearing: bool) {
    let (from_x, from_y) = (self.gaze_x, self.gaze_y);
    let hold = if let Some((back_x, back_y)) = self.glance_back.take() {
      // Done peeking, back to what it was looking at
      self.gaze_x = back_x;
      self.gaze_y = back_y;
      FIXATION_MIN + self.rng.rand_u32() % FIXATION_SPREAD
    } else if self.scan_left > 0 {
      self.scan_left -= 1;
      self.gaze_x = self.rand_signed() * SCAN_RANGE_X;
      self.gaze_y = self.rand_signed() * SACCADE_RANGE_Y;
      SCAN_HOLD_MIN + self.rng.rand_u32() % SCAN_HOLD_SPREAD
    } else if self.rng.rand_float() < MICROSACCADE_CHANCE {
      let dx = self.rand_signed() * MICROSACCADE_RANGE;
      let dy = self.rand_signed() * MICROSACCADE_RANGE;
      self.gaze_x = (self.gaze_x + dx).clamp(-SACCADE_RANGE_X, SACCADE_RANGE_X);
      self.gaze_y = (self.gaze_y + dy).clamp(-SACCADE_RANGE_Y, SACCADE_RANGE_Y);
      MICROSACCADE_HOLD_MIN + self.rng.rand_u32() % MICROSACCADE_HOLD_SPREAD
    } else {
      if !hearing
        && drowsy > YAWN_DROWSINESS
        && self.expr_mode == Expression::Normal
        && self.blink_phase == BlinkPhase::Open
        && self.rolls(crate::config::yawn_rarity())
      {
        self.trigger_expr(Expression::Yawn, 0.0);
      }
      let glance = if !hearing && self.rng.rand_float() < QUIRK_CHANCE {
        self.start_quirk()
      } else {
        None
      };
      glance.unwrap_or_else(|| {
        // Squared so it keeps coming home to the middle
        let rx = self.rand_signed();
        let ry = self.rand_signed();
        self.gaze_x = rx * rx.abs() * SACCADE_RANGE_X;
        self.gaze_y = ry * ry.abs() * SACCADE_RANGE_Y;
        FIXATION_MIN + self.rng.rand_u32() % FIXATION_SPREAD
      })
    };
    // Sleepy eyes stare longer
    self.gaze_hold = (hold as f32 * (1.0 + 2.0 * drowsy)) as u32;

    let jump = (self.gaze_x - from_x)
      .abs()
      .max((self.gaze_y - from_y).abs());
    if jump > SACCADE_BLINK_DISTANCE
      && self.blink_phase == BlinkPhase::Open
      && self.rng.rand_float() < SACCADE_BLINK_CHANCE
    {
      self.start_close();
    }
  }

  /// True with probability `rarity`% (0 = never)
  fn rolls(&mut self, rarity: u32) -> bool {
    rarity > 0 && self.rng.rand_u32() % RARITY_SCALE < rarity
  }

  /// Behind a closed lid nobody sees the Lamb slip into Sin now and then.
  /// Otherwise it settles back to `Normal`
  fn roll_idle_sin(&mut self) {
    let next = if self.rolls(crate::config::sin_rarity()) {
      Expression::Sin
    } else {
      Expression::Normal
    };
    self.trigger_expr(next, 0.0);
  }

  pub fn update(&mut self, dt_ms: f32) {
    self.phase += 0.038;

    let mode = self.expr_mode;
    let expr_active = mode.is_animated();
    let hearing = self.sound_x.abs() > 0.01;
    // A yawn is part of being bored, it doesn't wake anyone up
    self.quiet_ticks = if hearing || (expr_active && mode != Expression::Yawn) {
      0
    } else {
      // Capped at full drowsiness so a nod can knock it back down
      (self.quiet_ticks + 1).min(DROWSY_FULL_TICKS)
    };
    let drowsy = self.drowsiness();

    // Saccades always count down, even while sound is active
    if self.gaze_hold == 0 {
      self.next_fixation(drowsy, hearing);
    } else {
      self.gaze_hold -= 1;
    }

    let drift_x = DRIFT_AMPLITUDE * ((self.phase * 0.7).sin() + 0.5 * (self.phase * 1.9).sin());
    let drift_y =
      DRIFT_AMPLITUDE * ((self.phase * 0.5 + 1.0).sin() + 0.5 * (self.phase * 1.3).sin());
    let target_x = if hearing { self.sound_x } else { self.gaze_x } + drift_x;
    let target_y = self.gaze_y + drift_y;

    // Axes owned by an active expression skip the spring and lose their momentum
    if expr_active {
      self.gaze_vx = 0.0;
    } else {
      spring(&mut self.x, &mut self.gaze_vx, target_x);
    }
    if mode.owns_gaze_y() {
      self.gaze_vy = 0.0;
    } else {
      spring(&mut self.y, &mut self.gaze_vy, target_y);
    }

    // An open eye hands its lids over to the yawn
    if mode != Expression::Yawn || self.blink_phase != BlinkPhase::Open {
      self.update_blink(drowsy);
    }

    // Expression animations advance and set this frame's squint/pupil targets
    self.apply_expression(dt_ms);

    // Smoothly lerp expr_squint toward its target.
    // Fast in (tracking expression), slow out (eyelid drifts back after
    // expression)
    let squint_rate = if self.expr_squint_target >= self.expr_squint {
      0.025 // fast in
    } else {
      0.010 // slow out
    };
    self.expr_squint += (self.expr_squint_target - self.expr_squint) * squint_rate * dt_ms;

    // Smoothly lerp pupil scale
    let expressing = (self.expr_pupil_target - 1.0).abs() >= (self.expr_pupil_scale - 1.0).abs();
    let pupil_rate = if expressing { 0.025 } else { 0.010 };
    self.expr_pupil_scale += (self.expr_pupil_target - self.expr_pupil_scale) * pupil_rate * dt_ms;

    // Measured after expressions moved it too, so a shake stretches as well
    let speed_x = (self.x - self.prev_x).abs();
    let speed_y = (self.y - self.prev_y).abs() * STRETCH_Y_WEIGHT;
    (self.prev_x, self.prev_y) = (self.x, self.y);
    let stretch = (1.0 + STRETCH_GAIN * (speed_x - speed_y)).clamp(STRETCH_MIN, STRETCH_MAX);
    self.stretch += (stretch - self.stretch) * STRETCH_FOLLOW;
  }

  /// Blink state machine
  fn update_blink(&mut self, drowsy: f32) {
    match self.blink_phase {
      BlinkPhase::Open => {
        if self.blink_timer == 0 {
          self.start_close();
        } else {
          self.blink_timer -= 1;
        }
      }
      BlinkPhase::Closing => {
        // Ease in, the lid picks up speed on the way down
        self.lid_t = (self.lid_t + self.blink_speed).min(1.0);
        self.blink = 1.0 - self.lid_t * self.lid_t;
        if self.lid_t >= 1.0 {
          self.blink = 0.0;
          self.nodding = drowsy > NOD_OFF_DROWSINESS && self.rng.rand_float() < NOD_OFF_CHANCE;
          let hold = if self.nodding {
            NOD_OFF_HOLD_MIN + self.rng.rand_u32() % NOD_OFF_HOLD_SPREAD
          } else {
            BLINK_HOLD_MIN
              + self.rng.rand_u32() % BLINK_HOLD_SPREAD
              + (drowsy * DROWSY_BLINK_HOLD) as u32
          };
          self.blink_phase = BlinkPhase::Closed(hold);
        }
      }
      BlinkPhase::Closed(remaining_frames) => {
        self.dark = (self.dark + 1.0 / DARK_FULL_TICKS).min(1.0);
        if remaining_frames == 0 {
          self.reflex = LIGHT_REFLEX_MIN + (1.0 - LIGHT_REFLEX_MIN) * self.dark;
          self.dark = 0.0;
          // A queued UI expression goes first, otherwise maybe slip into Sin
          if let Some((mode, param)) = self.pending_expr.take() {
            self.trigger_expr(mode, param);
          } else if !self.expr_mode.is_animated() {
            self.roll_idle_sin();
          }
          self.lid_t = 0.0;
          self.blink_speed = if self.waking {
            WAKE_OPEN_SPEED
          } else if self.nodding {
            // Caught itself dozing, eyes snap open and it's half as sleepy
            self.quiet_ticks -= (DROWSY_RAMP_TICKS / 2.0) as u32;
            NOD_OFF_OPEN_SPEED
          } else {
            // Sleepy lids drag themselves back up
            BLINK_OPEN_SPEED * self.jitter() * (1.0 - 0.5 * drowsy)
          };
          self.blink_phase = BlinkPhase::Opening;
        } else {
          self.blink_phase = BlinkPhase::Closed(remaining_frames - 1);
        }
      }
      BlinkPhase::Opening => {
        // Ease out, quick off the lash line then settles
        self.lid_t = (self.lid_t + self.blink_speed).min(1.0);
        let rest = 1.0 - self.lid_t;
        self.blink = 1.0 - rest * rest;
        if self.lid_t >= 1.0 {
          self.blink = 1.0;
          self.blink_timer = if self.waking {
            // Rubs the sleep out with a quick second blink
            self.waking = false;
            WAKE_DOUBLE_BLINK_GAP
          } else if self.double_blink || self.rng.rand_float() < DOUBLE_BLINK_CHANCE {
            self.double_blink = false;
            DOUBLE_BLINK_GAP
          } else {
            crate::config::blink_interval() + self.rng.rand_u32() % BLINK_INTERVAL_RANGE
          };
          self.blink_phase = BlinkPhase::Open;
        }
      }
    }
  }

  /// Advances the active expression animation and sets this frame's squint and
  /// pupil targets. Targets are reset first, so callers always get a value even
  /// when no expression is playing
  fn apply_expression(&mut self, dt_ms: f32) {
    use core::f32::consts::PI;

    // Defaults, sleepy lids droop and the pupil breathes
    self.expr_squint_target = self.drowsiness() * DROWSY_SQUINT;
    self.reflex *= LIGHT_REFLEX_DECAY;
    let curious = self.curious_ticks as f32 / CURIOUS_TICKS as f32;
    self.curious_ticks = self.curious_ticks.saturating_sub(1);
    self.expr_pupil_target = 1.0
      + PUPIL_BREATH * (self.phase * 0.21).sin()
      + CURIOUS_PUPIL * (curious * core::f32::consts::PI).sin()
      + DARK_DILATE * self.dark
      - LIGHT_REFLEX_SHRINK * self.reflex;

    let mode = self.expr_mode;
    if !mode.is_animated() {
      return;
    }

    let progress = self.expr_t;
    match mode {
      Expression::EyeRoll => {
        let angle = progress * 2.0 * PI;
        self.x = angle.sin() * 0.72;
        self.y = -(1.0 - angle.cos()) / 2.0;
        self.expr_squint_target = (progress * PI).sin() * 0.70;
      }
      Expression::Startled => {
        // Lerp toward target instead of direct assignment
        let fade = if progress > 0.65 {
          1.0 - (progress - 0.65) / 0.35
        } else {
          1.0
        };
        let tx = (progress * 70.0).sin() * 0.55 * fade;
        let ty = (progress * 55.0 + 1.5).sin() * 0.38 * fade;
        self.x += (tx - self.x) * 0.35;
        self.y += (ty - self.y) * 0.35;
        self.expr_pupil_target = 1.0 - fade * 0.85;
      }
      Expression::Suspicious => {
        // Quick snap to side, eye squints heavily, then slowly reveals
        let move_t = (progress / 0.08).min(1.0);
        self.x = self.expr_param * move_t + (progress * 9.0).sin() * 0.025 * move_t;
        let squint = if progress < 0.15 {
          (progress / 0.15) * 0.75 // fast squint in
        } else {
          let reveal = ((progress - 0.15) / 0.40).min(1.0);
          0.75 - reveal * 0.45 // slow reveal -> 0.30
        };
        self.expr_squint_target = squint;
        self.expr_pupil_target = 1.0 - squint * 0.3; // shrink to 0.7× along with the squint
      }
      Expression::Angry => {
        let squint = (progress / 0.12).min(1.0);
        self.expr_squint_target = squint * 0.55;
        self.expr_pupil_target = 1.0 - squint * 0.4; // shrink to 0.6× along with the anger
        let shake_amp = if progress > 0.7 {
          (1.0 - progress) / 0.3 * 0.5
        } else {
          0.5
        };
        let ax = (progress * 42.0).sin() * shake_amp;
        self.x += (ax - self.x) * 0.35;
      }
      Expression::Yawn => self.play_yawn(progress),
      Expression::Delight => self.play_delight(progress),
      Expression::Dizzy => self.play_dizzy(progress),
      Expression::Normal | Expression::Sin => {}
    }

    self.expr_t += mode.animation_speed() * dt_ms;
    if self.expr_t >= 1.0 {
      self.expr_t = 0.0;
      self.set_mode(Expression::Normal);
      self.finish_expression(mode);
    }
  }

  /// Lids sink most of the way and the eye drops a bit, then a lazy reopen
  fn play_yawn(&mut self, progress: f32) {
    let sink = smoothstep(progress / 0.45) * (1.0 - smoothstep((progress - 0.6) / 0.4));
    if self.blink_phase == BlinkPhase::Open {
      self.blink = 1.0 - YAWN_CLOSE * sink;
    }
    self.x -= self.x * 0.03;
    self.y += (YAWN_LOOK_DOWN * sink - self.y) * 0.1;
    self.expr_pupil_target = 1.0 + YAWN_PUPIL * sink;
  }

  /// Big pupil, wide open and two hops like it can't sit still
  fn play_delight(&mut self, progress: f32) {
    use core::f32::consts::PI;
    self.x -= self.x * 0.15;
    self.y = -(progress * 2.0 * PI).sin().abs() * DELIGHT_HOP * (1.0 - progress);
    self.expr_squint_target = 0.0;
    self.expr_pupil_target = 1.0 + DELIGHT_PUPIL * (progress * PI).sin().sqrt();
  }

  /// Spins round in circles that swell and fade, pupil wobbling along
  fn play_dizzy(&mut self, progress: f32) {
    use core::f32::consts::PI;
    let swell = (progress * PI).sin();
    let angle = progress * DIZZY_TURNS * 2.0 * PI;
    let radius = DIZZY_RADIUS * swell;
    self.x += (angle.cos() * radius - self.x) * 0.3;
    self.y += (angle.sin() * radius - self.y) * 0.3;
    self.expr_squint_target = DIZZY_SQUINT * swell;
    self.expr_pupil_target = 1.0 + DIZZY_WOBBLE * (progress * 40.0).sin() * swell;
  }

  /// Some moods end with a blink, like shaking it off
  fn finish_expression(&mut self, mode: Expression) {
    if self.blink_phase != BlinkPhase::Open {
      return;
    }
    match mode {
      Expression::Yawn => self.blink_timer = YAWN_BLINK_DELAY,
      Expression::Delight => {
        self.blink_timer = 0;
        self.double_blink = true;
      }
      Expression::Dizzy => self.blink_timer = 0,
      _ => {}
    }
  }
}

/// Eases 0..1 in and out, anything outside gets clamped
fn smoothstep(progress: f32) -> f32 {
  let progress = progress.clamp(0.0, 1.0);
  progress * progress * (3.0 - 2.0 * progress)
}

/// One semi-implicit Euler step of a damped spring
fn spring(pos: &mut f32, vel: &mut f32, target: f32) {
  *vel += (target - *pos) * GAZE_STIFFNESS - *vel * GAZE_DAMPING;
  *pos += *vel;
}

#[cfg(test)]
mod tests {
  use super::{Expression, EyeState};

  const WEB_CODES: [i32; 9] = [1, 2, 3, 4, 5, 7, 8, 9, 10];

  #[test]
  fn from_web_code_accepts_exactly_the_documented_codes() {
    for code in WEB_CODES {
      assert!(
        Expression::from_web_code(code).is_some(),
        "code {code} should map to an expression"
      );
    }
  }

  #[test]
  fn from_web_code_rejects_everything_else() {
    for code in -300..300 {
      if WEB_CODES.contains(&code) {
        continue;
      }
      assert!(
        Expression::from_web_code(code).is_none(),
        "code {code} must not map to an expression"
      );
    }
  }

  #[test]
  fn from_web_code_never_maps_two_codes_to_one_expression() {
    for (index, left) in WEB_CODES.iter().enumerate() {
      for right in &WEB_CODES[index + 1..] {
        assert!(
          Expression::from_web_code(*left) != Expression::from_web_code(*right),
          "codes {left} and {right} map to the same expression"
        );
      }
    }
  }

  #[test]
  fn resting_expressions_do_not_animate() {
    for mode in [Expression::Normal, Expression::Sin] {
      assert!(!mode.is_animated());
      assert!(
        mode.animation_speed().abs() < f32::EPSILON,
        "a resting expression must not advance expr_t"
      );
    }
  }

  #[test]
  fn animated_expressions_advance() {
    for mode in [
      Expression::EyeRoll,
      Expression::Startled,
      Expression::Suspicious,
      Expression::Angry,
      Expression::Yawn,
      Expression::Delight,
      Expression::Dizzy,
    ] {
      assert!(mode.is_animated());
      assert!(
        mode.animation_speed() > 0.0,
        "an animated expression that never advances would stick forever"
      );
    }
  }

  #[test]
  fn trigger_expr_restarts_the_animation_and_drops_the_queue() {
    let mut eye = EyeState::new(1);
    eye.queue_expr(Expression::Angry, 0.5);
    eye.trigger_expr(Expression::Startled, 0.25);

    assert!(eye.pending_expr.is_none(), "trigger must cancel the queue");
    assert!(eye.expr_mode == Expression::Startled);
    assert!(
      eye.expr_t.abs() < f32::EPSILON,
      "a new expression starts from the top"
    );
    assert!((eye.expr_param - 0.25).abs() < f32::EPSILON);
  }

  #[test]
  fn queue_expr_starts_closing_the_lid_from_any_open_phase() {
    for phase in [
      super::BlinkPhase::Open,
      super::BlinkPhase::Opening,
      super::BlinkPhase::Closing,
    ] {
      let mut eye = EyeState::new(2);
      eye.blink_phase = phase;
      eye.queue_expr(Expression::EyeRoll, 0.0);

      assert!(
        eye.pending_expr.is_some(),
        "the expression must stay queued until the lid closes"
      );
      assert!(
        matches!(eye.blink_phase, super::BlinkPhase::Closing),
        "a queued expression is applied at the blink close, so the lid has to get there"
      );
    }
  }

  #[test]
  fn update_keeps_blink_within_its_documented_range() {
    let mut eye = EyeState::new(3);

    // Long enough to walk through several full blink cycles.
    for _ in 0..5_000 {
      eye.update(16.0);
      assert!(
        (0.0..=1.0).contains(&eye.blink),
        "blink escaped 0..=1 at {}",
        eye.blink
      );
      assert!(
        (0.0..=1.0).contains(&eye.expr_t),
        "expr_t escaped 0..=1 at {}",
        eye.expr_t
      );
      assert!(
        eye.expr_pupil_scale > 0.0,
        "a pupil scale of 0 would vanish"
      );
    }
  }

  #[test]
  fn a_long_silence_makes_the_eye_drowsy_but_it_keeps_blinking_and_looking() {
    let mut eye = EyeState::new(4);
    let (mut drowsiest, mut droopiest, mut yawned) = (0.0_f32, 0.0_f32, false);
    for _ in 0..super::DROWSY_FULL_TICKS {
      eye.update(16.0);
      drowsiest = drowsiest.max(eye.drowsiness());
      droopiest = droopiest.max(eye.expr_squint);
      yawned |= eye.expr_mode == Expression::Yawn;
    }
    assert!(drowsiest > 0.6, "silence should make it sleepy");
    assert!(droopiest > 0.2, "drowsy lids should droop");
    assert!(yawned, "a bored Lamb should yawn now and then");

    let (mut blinked, mut min_x, mut max_x) = (false, f32::MAX, f32::MIN);
    for _ in 0..3_000 {
      eye.update(16.0);
      blinked |= eye.blink < 0.01;
      min_x = min_x.min(eye.x);
      max_x = max_x.max(eye.x);
      assert!(
        eye.x.abs() < 1.0 && eye.y.abs() < 1.0,
        "gaze left the screen"
      );
    }
    assert!(blinked, "a sleepy eye still blinks");
    assert!(max_x - min_x > 0.05, "a sleepy eye still looks around");

    eye.sound_x = 0.5;
    eye.update(16.0);
    assert!(eye.drowsiness() < f32::EPSILON, "a sound wakes it up");
  }

  #[test]
  fn boot_starts_with_the_lid_shut_and_opens_it() {
    let mut eye = EyeState::new(5);
    assert!(
      eye.blink < f32::EPSILON,
      "the first frame should be a closed eye"
    );

    let opened_at = (0..600)
      .find(|_| {
        eye.update(16.0);
        !eye.waking
      })
      .expect("the eye never opened after boot");
    assert!(
      opened_at > 40,
      "a greeting should be slow, opened after {opened_at} ticks"
    );
    assert!(eye.blink >= 1.0, "the greeting ends wide open");
  }

  #[test]
  fn the_pupil_flinches_at_the_light_and_stretches_on_saccades() {
    let mut eye = EyeState::new(6);
    let (mut smallest, mut widest) = (f32::MAX, 0.0_f32);
    for _ in 0..3_000 {
      eye.update(16.0);
      if !eye.waking {
        smallest = smallest.min(eye.expr_pupil_scale);
      }
      widest = widest.max(eye.stretch);
      assert!(
        (super::STRETCH_MIN..=super::STRETCH_MAX).contains(&eye.stretch),
        "stretch escaped its clamp at {}",
        eye.stretch
      );
    }
    assert!(
      smallest < 0.95,
      "opening up should make the pupil flinch, got {smallest}"
    );
    assert!(
      widest > 1.1,
      "a saccade should stretch the pupil, got {widest}"
    );
  }

  /// Settles past the boot greeting, then plays `mode` to the end
  fn play(mode: Expression, mut each_tick: impl FnMut(&EyeState)) -> EyeState {
    let mut eye = EyeState::new(8);
    for _ in 0..400 {
      eye.update(16.0);
    }
    eye.quiet_ticks = super::DROWSY_FULL_TICKS;
    eye.blink_phase = super::BlinkPhase::Open;
    eye.blink = 1.0;
    eye.trigger_expr(mode, 0.0);
    while eye.expr_mode == mode {
      eye.update(16.0);
      each_tick(&eye);
    }
    eye
  }

  #[test]
  fn yawn_sinks_the_lids_without_waking_it_up() {
    let mut lowest = 1.0_f32;
    let eye = play(Expression::Yawn, |eye| lowest = lowest.min(eye.blink));
    assert!(lowest < 0.3, "the lids should sink, got {lowest}");
    assert!(
      eye.drowsiness() > 0.9,
      "a yawn shouldn't shake off the sleep"
    );
    assert!(
      eye.blink_timer <= super::YAWN_BLINK_DELAY,
      "a blink should finish it off"
    );
  }

  #[test]
  fn delight_hops_with_a_big_pupil() {
    let (mut biggest, mut highest) = (0.0_f32, 0.0_f32);
    let eye = play(Expression::Delight, |eye| {
      biggest = biggest.max(eye.expr_pupil_scale);
      highest = highest.min(eye.y);
    });
    assert!(biggest > 1.3, "the pupil should blow up, got {biggest}");
    assert!(highest < -0.2, "it should hop, got {highest}");
    assert!(eye.double_blink, "a double blink should follow");
  }

  #[test]
  fn dizzy_spins_all_the_way_round() {
    let (mut left, mut right, mut up, mut down) = (0.0_f32, 0.0_f32, 0.0_f32, 0.0_f32);
    play(Expression::Dizzy, |eye| {
      (left, right) = (left.min(eye.x), right.max(eye.x));
      (up, down) = (up.min(eye.y), down.max(eye.y));
    });
    for (side, reach) in [
      ("left", -left),
      ("right", right),
      ("up", -up),
      ("down", down),
    ] {
      assert!(reach > 0.2, "the spin never went {side}, reached {reach}");
    }
  }
}
