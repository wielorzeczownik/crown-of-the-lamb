#[cfg_attr(not(target_arch = "xtensa"), allow(unused_imports))]
use micromath::F32Ext;
use oorandom::Rand32;

use crate::constants::RARITY_SCALE;

// Idle look-around sequence: normalised positions -1..1 and hold frames
const IDLE_SEQ_X: [f32; 8] = [0.0, -0.3, 0.0, 0.3, 0.0, -0.2, 0.2, 0.0];
const IDLE_SEQ_Y: [f32; 8] = [0.0, -0.15, 0.0, 0.15, 0.0, -0.1, 0.1, 0.0];
const IDLE_HOLD: [u32; 8] = [180, 120, 150, 120, 200, 120, 120, 150];

// Blink timing
const BLINK_CLOSE_SPEED: f32 = 0.18;
const BLINK_OPEN_SPEED: f32 = 0.14;
const BLINK_HOLD_FRAMES: u32 = 2;
const BLINK_INTERVAL_MIN: u32 = 180;
const BLINK_INTERVAL_RANGE: u32 = 300;

// expr_t advances this much per frame (1 ms); reaches 1.0 = expression done
const EXPR_SPEED_EYEROLL: f32 = 1.0 / 1500.0; // 1.5 s full eye-roll arc
const EXPR_SPEED_STARTLED: f32 = 1.0 / 1400.0; // 1.4 s slower tremor
const EXPR_SPEED_SUSPICIOUS: f32 = 1.0 / 3000.0; // 3 s hold + squint reveal
const EXPR_SPEED_ANGRY: f32 = 1.0 / 2000.0; // 2 s shake + squint

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
      _ => return None,
    })
  }

  /// Resting states (Normal, Sin) just hold; everything else is a timed
  /// animation driven by expr_t
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
      Self::Normal | Self::Sin => 0.0,
    }
  }
}

pub struct EyeState {
  pub x: f32,
  pub y: f32,
  pub phase: f32, // phase for pupil edge wiggle
  pub blink: f32, // 1.0 = fully open, 0.0 = closed
  pub blink_phase: BlinkPhase,
  pub blink_timer: u32,
  pub idle_seq: usize,
  pub idle_hold: u32,
  pub sound_x: f32,          // -1..1, set by sound detection
  pub extra_close: f32,      // sound squint
  pub expr_squint: f32,      // expression squint
  pub expr_pupil_scale: f32, // pupil dilation scale
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
      blink: 1.0,
      blink_phase: BlinkPhase::Open,
      blink_timer: BLINK_INTERVAL_MIN + 80,
      idle_seq: 0,
      idle_hold: IDLE_HOLD[0],
      sound_x: 0.0,
      extra_close: 0.0,
      expr_squint: 0.0,
      expr_pupil_scale: 1.0,
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
      BlinkPhase::Open | BlinkPhase::Opening => {
        self.blink_phase = BlinkPhase::Closing;
      }
      _ => {} // already closing/closed
    }
  }

  /// At an idle blink close (nothing queued or playing), occasionally drop into
  /// the Sin resting state otherwise settle back to `Normal`
  fn roll_idle_sin(&mut self) {
    let rarity = crate::config::sin_rarity();
    let next = if rarity > 0 && self.rng.rand_u32() % RARITY_SCALE < rarity {
      Expression::Sin
    } else {
      Expression::Normal
    };
    self.trigger_expr(next, 0.0);
  }

  pub fn update(&mut self, dt_ms: f32) {
    self.phase += 0.038;

    // Idle look-around always counts down, even while sound is active
    if self.idle_hold == 0 {
      self.idle_seq = (self.idle_seq + 1) % IDLE_SEQ_X.len();
      self.idle_hold = IDLE_HOLD[self.idle_seq];
    } else {
      self.idle_hold -= 1;
    }
    let target_y = IDLE_SEQ_Y[self.idle_seq];

    let (target_x, lerp_x) = if self.sound_x.abs() > 0.01 {
      (self.sound_x, 0.08)
    } else {
      (IDLE_SEQ_X[self.idle_seq], 0.05)
    };

    // Skip idle lerp for axes controlled by an active expression
    let mode = self.expr_mode;
    let expr_active = mode.is_animated();
    if !expr_active {
      self.x += (target_x - self.x) * lerp_x;
    }
    if !matches!(mode, Expression::EyeRoll | Expression::Startled) {
      self.y += (target_y - self.y) * 0.04;
    }

    // Blink state machine
    match self.blink_phase {
      BlinkPhase::Open => {
        if self.blink_timer == 0 {
          self.blink_phase = BlinkPhase::Closing;
        } else {
          self.blink_timer -= 1;
        }
      }
      BlinkPhase::Closing => {
        self.blink -= BLINK_CLOSE_SPEED;
        if self.blink <= 0.0 {
          self.blink = 0.0;
          self.blink_phase = BlinkPhase::Closed(BLINK_HOLD_FRAMES);
        }
      }
      BlinkPhase::Closed(remaining_frames) => {
        if remaining_frames == 0 {
          // Apply a pending UI-triggered expression first; otherwise run the
          // sin randomisation that normally lives behind the closed lid.
          if let Some((mode, param)) = self.pending_expr.take() {
            self.trigger_expr(mode, param);
          } else if !self.expr_mode.is_animated() {
            self.roll_idle_sin();
          }
          self.blink_phase = BlinkPhase::Opening;
        } else {
          self.blink_phase = BlinkPhase::Closed(remaining_frames - 1);
        }
      }
      BlinkPhase::Opening => {
        self.blink += BLINK_OPEN_SPEED;
        if self.blink >= 1.0 {
          self.blink = 1.0;
          self.blink_timer =
            crate::config::blink_interval() + self.rng.rand_u32() % BLINK_INTERVAL_RANGE;
          self.blink_phase = BlinkPhase::Open;
        }
      }
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
  }

  /// Advances the active expression animation and sets this frame's squint and
  /// pupil targets. Targets are reset first, so callers always get a value even
  /// when no expression is playing
  fn apply_expression(&mut self, dt_ms: f32) {
    use core::f32::consts::PI;

    self.expr_squint_target = 0.0; // default: no expression squint this frame
    self.expr_pupil_target = 1.0; // default: normal pupil size

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
      Expression::Normal | Expression::Sin => {}
    }

    self.expr_t += mode.animation_speed() * dt_ms;
    if self.expr_t >= 1.0 {
      self.expr_t = 0.0;
      self.set_mode(Expression::Normal);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{Expression, EyeState};

  const WEB_CODES: [i32; 6] = [1, 2, 3, 4, 5, 7];

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
}
