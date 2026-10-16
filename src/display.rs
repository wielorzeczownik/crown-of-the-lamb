use embedded_graphics::pixelcolor::{IntoStorage, Rgb565};
#[cfg_attr(not(target_arch = "xtensa"), allow(unused_imports))]
use micromath::F32Ext;

use crate::eye::EyeState;

// Display geometry
pub const DISPLAY_W: i32 = 240;
pub const DISPLAY_H: i32 = 240;
const DISPLAY_CENTER: i32 = DISPLAY_W / 2;
// One row of RGB565 pixels
pub const ROW_PIXELS: usize = DISPLAY_W as usize;
pub const ROW_BYTES: usize = ROW_PIXELS * 2;
// Pupil half-width (horizontal axis) and max pupil offset (px)
const PUPIL_A: i32 = 12;
const EYE_MOVE_X: f32 = 35.0;
const EYE_MOVE_Y: f32 = 20.0;
// Specular highlight: offset from the pupil centre, ellipse weights, threshold
const HIGHLIGHT_OFFSET_X: i32 = 22;
const HIGHLIGHT_OFFSET_Y: i32 = -26;
const HIGHLIGHT_WEIGHT_X: i32 = 3;
const HIGHLIGHT_WEIGHT_Y: i32 = 8;
const HIGHLIGHT_THRESHOLD: i32 = 150;
// Highlight half-width (3 * 7^2 <= 150)
const HIGHLIGHT_MAX_DX: i32 = 7;
// Radial gradient falloff from centre (1.0) to display edge
const GRADIENT_FALLOFF_RB: f32 = 0.46; // red & blue
const GRADIENT_FALLOFF_G: f32 = 0.80; // green falls off faster (warmer edge)
// Gradient table index is squared distance >> shift
const GRADIENT_LUT_SHIFT: i32 = 4;
const GRADIENT_LUT_LEN: usize = (DISPLAY_RADIUS_SQ >> GRADIENT_LUT_SHIFT) as usize + 1;
// Extra eyelid close from a full squint, on top of the blink amount
const SQUINT_BLINK_FACTOR: f32 = 0.65;
// Second wiggle harmonic contributes half the first's amplitude
const WIGGLE_SECOND_HARMONIC: f32 = 0.5;
// Sin eye half-width
const SIN_EYE_MAX_DX: i32 = 4;

const DISPLAY_RADIUS_SQ: i32 = DISPLAY_CENTER * DISPLAY_CENTER; // for radial gradient

const BLACK: u16 = 0;
const HIGHLIGHT: Rgb565 = Rgb565::new(31, 55, 22);

use crate::sin_bitmap::{SIN_BMP, SIN_BMP_H, SIN_BMP_ROW, SIN_BMP_W, SIN_EYE_X, SIN_EYE_Y};

/// Pixel with big-endian bytes in memory, as the panel wants them
fn wire(color: Rgb565) -> u16 {
  color.into_storage().to_be()
}

#[inline]
fn in_sin_bitmap(rx: i32, ry: i32) -> bool {
  let bx = rx + SIN_BMP_W / 2;
  let by = ry + SIN_BMP_H / 2;
  if !(0..SIN_BMP_W).contains(&bx) || !(0..SIN_BMP_H).contains(&by) {
    return false;
  }
  let byte_idx = by as usize * SIN_BMP_ROW + bx as usize / 8;
  let bit_pos = 7 - (bx as usize % 8);
  SIN_BMP[byte_idx] & (1 << bit_pos) != 0
}

// 4x8 px oval pupil
#[inline]
fn in_sin_eye(rx: i32, ry: i32) -> bool {
  let dx = rx - SIN_EYE_X;
  let dy = ry - SIN_EYE_Y;
  dx * dx * 4 + dy * dy <= 64
}

// 2x7 px vertical slit (narrower than the eye, leaving the coloured rim)
#[inline]
fn in_sin_eye_slit(rx: i32, ry: i32) -> bool {
  let dx = rx - SIN_EYE_X;
  let dy = ry - SIN_EYE_Y;
  dx * dx * 49 + dy * dy * 4 <= 196
}

/// Paints `color` over columns `from..=to`, clipped to the row
fn fill_span(out: &mut [u16], from: i32, to: i32, color: u16) {
  let from = from.max(0) as usize;
  let to = to.min(DISPLAY_W - 1);
  if to < 0 || from > to as usize {
    return;
  }
  out[from..=to as usize].fill(color);
}

/// Per-frame values, computed once
pub struct Frame {
  center_x: i32,
  center_y: i32,
  open_half_h: i32,
  highlight_x: i32,
  highlight_y: i32,
  pupil_height: i32,
  pupil_a: f32,
  wiggle_amp: f32,
  phase: f32,
  sin_mode: bool,
  eye_color: u16,
  highlight: u16,
  gradient: [u16; GRADIENT_LUT_LEN],
}

impl Frame {
  pub fn new(eye: &EyeState) -> Self {
    Self::from_parts(
      eye,
      crate::config::pupil_height(),
      crate::config::wiggle_amplitude(),
      [
        crate::config::eye_color_red(),
        crate::config::eye_color_green(),
        crate::config::eye_color_blue(),
      ],
    )
  }

  fn from_parts(eye: &EyeState, pupil_height: i32, wiggle_amp: f32, rgb: [f32; 3]) -> Self {
    let center_x = DISPLAY_CENTER + (eye.x * EYE_MOVE_X) as i32;
    let center_y = DISPLAY_CENTER + (eye.y * EYE_MOVE_Y) as i32;

    // Combine sound squint and expression squint,
    // both smoothly lerped, capped so they can't exceed full-squint
    let squint = (eye.extra_close + eye.expr_squint).min(1.0);
    let effective_blink = eye.blink * (1.0 - squint * SQUINT_BLINK_FACTOR);

    let [red, green, blue] = rgb;
    let mut gradient = [BLACK; GRADIENT_LUT_LEN];
    for (idx, entry) in gradient.iter_mut().enumerate() {
      // Hollow-Face radial gradient: bright centre, dark edge.
      let dist = (idx << GRADIENT_LUT_SHIFT) as f32;
      let norm = (dist * (1.0 / DISPLAY_RADIUS_SQ as f32)).min(1.0);
      *entry = wire(Rgb565::new(
        (red * (1.0 - GRADIENT_FALLOFF_RB * norm)) as u8,
        (green * (1.0 - GRADIENT_FALLOFF_G * norm)) as u8,
        (blue * (1.0 - GRADIENT_FALLOFF_RB * norm)) as u8,
      ));
    }

    Self {
      center_x,
      center_y,
      open_half_h: (DISPLAY_CENTER as f32 * effective_blink) as i32,
      // Highlight spot, offset right and up from the pupil centre
      highlight_x: center_x + HIGHLIGHT_OFFSET_X,
      highlight_y: center_y + HIGHLIGHT_OFFSET_Y,
      pupil_height: (pupil_height as f32 * eye.expr_pupil_scale) as i32,
      pupil_a: PUPIL_A as f32 * eye.expr_pupil_scale,
      wiggle_amp,
      phase: eye.phase,
      sin_mode: eye.expr_mode == crate::eye::Expression::Sin,
      eye_color: wire(Rgb565::new(red as u8, green as u8, blue as u8)),
      highlight: wire(HIGHLIGHT),
      gradient,
    }
  }

  /// Draws gradient, highlight, then pupil or Sin, in wire byte order
  pub fn render_row(&self, row: i32, out: &mut [u16]) {
    let out = &mut out[..ROW_PIXELS];
    let delta_y = row - DISPLAY_CENTER;
    if delta_y.abs() > self.open_half_h {
      out.fill(0);
      return;
    }

    let delta_y_sq = delta_y * delta_y;
    for (col, px) in (0..DISPLAY_W).zip(out.iter_mut()) {
      let col_offset = col - DISPLAY_CENTER;
      let dist = (col_offset * col_offset + delta_y_sq).min(DISPLAY_RADIUS_SQ);
      *px = self.gradient[(dist >> GRADIENT_LUT_SHIFT) as usize];
    }

    let row_from_spot = row - self.highlight_y;
    let row_from_spot_sq8 = row_from_spot * row_from_spot * HIGHLIGHT_WEIGHT_Y;
    if row_from_spot_sq8 <= HIGHLIGHT_THRESHOLD {
      for dx in -HIGHLIGHT_MAX_DX..=HIGHLIGHT_MAX_DX {
        if dx * dx * HIGHLIGHT_WEIGHT_X + row_from_spot_sq8 <= HIGHLIGHT_THRESHOLD {
          let col = self.highlight_x + dx;
          fill_span(out, col, col, self.highlight);
        }
      }
    }

    let rel_y = row - self.center_y;
    if self.sin_mode {
      self.paint_sin(out, rel_y);
    } else {
      self.paint_pupil(out, row, rel_y);
    }
  }

  fn paint_pupil(&self, out: &mut [u16], row: i32, rel_y: i32) {
    let norm_t = rel_y as f32 / self.pupil_height as f32;
    let norm_t_sq = norm_t * norm_t;
    if norm_t_sq >= 1.0 {
      return;
    }
    let pupil_half_w = self.pupil_a * (1.0_f32 - norm_t_sq).sqrt();
    if pupil_half_w <= 0.0 {
      return;
    }

    // Two independent sine signals per side, giving asymmetric organic wiggle
    let (phase, amp, row_f) = (self.phase, self.wiggle_amp, row as f32);
    let w_left = (phase * 0.73 + row_f * 0.14).sin() * amp
      + (phase * 2.1 + row_f * 0.31).sin() * (amp * WIGGLE_SECOND_HARMONIC);
    let w_right = (phase * 0.89 + row_f * 0.16 + 1.1).sin() * amp
      + (phase * 1.75 + row_f * 0.28 + 2.3).sin() * (amp * WIGGLE_SECOND_HARMONIC);

    let left_edge = (pupil_half_w + w_left).max(0.0) as i32;
    let right_edge = (pupil_half_w + w_right).max(0.0) as i32;
    if left_edge > 0 {
      fill_span(out, self.center_x - left_edge, self.center_x, BLACK);
    }
    if right_edge > 0 {
      fill_span(out, self.center_x + 1, self.center_x + right_edge, BLACK);
    }
  }

  fn paint_sin(&self, out: &mut [u16], rel_y: i32) {
    let half_w = SIN_BMP_W / 2;
    for rel_x in -half_w..half_w {
      if in_sin_bitmap(rel_x, rel_y) {
        let col = self.center_x + rel_x;
        fill_span(out, col, col, BLACK);
      }
    }
    for rel_x in SIN_EYE_X - SIN_EYE_MAX_DX..=SIN_EYE_X + SIN_EYE_MAX_DX {
      if in_sin_eye(rel_x, rel_y) {
        let color = if in_sin_eye_slit(rel_x, rel_y) {
          BLACK
        } else {
          self.eye_color
        };
        let col = self.center_x + rel_x;
        fill_span(out, col, col, color);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::{
    DISPLAY_CENTER, DISPLAY_H, DISPLAY_RADIUS_SQ, DISPLAY_W, EYE_MOVE_X, EYE_MOVE_Y, Frame,
    GRADIENT_FALLOFF_G, GRADIENT_FALLOFF_RB, HIGHLIGHT_OFFSET_X, HIGHLIGHT_OFFSET_Y,
    HIGHLIGHT_THRESHOLD, HIGHLIGHT_WEIGHT_X, HIGHLIGHT_WEIGHT_Y, PUPIL_A, ROW_PIXELS,
    SQUINT_BLINK_FACTOR, WIGGLE_SECOND_HARMONIC, in_sin_bitmap, in_sin_eye, in_sin_eye_slit,
  };
  use crate::eye::{Expression, EyeState};

  const PUPIL_HEIGHT: i32 = 70;
  const WIGGLE: f32 = 1.4;
  const RGB: [f32; 3] = [26.0, 40.0, 9.0];

  /// The original per-pixel float renderer, kept as the visual reference
  fn reference_pixel(eye: &EyeState, col: i32, row: i32) -> [i32; 3] {
    let center_x = DISPLAY_CENTER + (eye.x * EYE_MOVE_X) as i32;
    let center_y = DISPLAY_CENTER + (eye.y * EYE_MOVE_Y) as i32;
    let squint = (eye.extra_close + eye.expr_squint).min(1.0);
    let open_half_h =
      (DISPLAY_CENTER as f32 * eye.blink * (1.0 - squint * SQUINT_BLINK_FACTOR)) as i32;
    if (row - DISPLAY_CENTER).abs() > open_half_h {
      return [0; 3];
    }
    let (rel_x, rel_y) = (col - center_x, row - center_y);
    let [red_c, green_c, blue_c] = RGB;

    if eye.expr_mode == Expression::Sin {
      if in_sin_eye(rel_x, rel_y) {
        return if in_sin_eye_slit(rel_x, rel_y) {
          [0; 3]
        } else {
          [red_c as i32, green_c as i32, blue_c as i32]
        };
      }
      if in_sin_bitmap(rel_x, rel_y) {
        return [0; 3];
      }
    } else {
      let norm_t = rel_y as f32 / (PUPIL_HEIGHT as f32 * eye.expr_pupil_scale) as i32 as f32;
      let norm_t_sq = norm_t * norm_t;
      let half_w = if norm_t_sq >= 1.0 {
        0.0
      } else {
        PUPIL_A as f32 * eye.expr_pupil_scale * (1.0 - norm_t_sq).sqrt()
      };
      let (phase, row_f) = (eye.phase, row as f32);
      let w_left = (phase * 0.73 + row_f * 0.14).sin() * WIGGLE
        + (phase * 2.1 + row_f * 0.31).sin() * (WIGGLE * WIGGLE_SECOND_HARMONIC);
      let w_right = (phase * 0.89 + row_f * 0.16 + 1.1).sin() * WIGGLE
        + (phase * 1.75 + row_f * 0.28 + 2.3).sin() * (WIGGLE * WIGGLE_SECOND_HARMONIC);
      let left = if half_w > 0.0 {
        (half_w + w_left).max(0.0) as i32
      } else {
        0
      };
      let right = if half_w > 0.0 {
        (half_w + w_right).max(0.0) as i32
      } else {
        0
      };
      if rel_x <= 0 && left > 0 && -rel_x <= left {
        return [0; 3];
      }
      if rel_x > 0 && right > 0 && rel_x <= right {
        return [0; 3];
      }
    }

    let spot_x = col - (center_x + HIGHLIGHT_OFFSET_X);
    let spot_y = row - (center_y + HIGHLIGHT_OFFSET_Y);
    if spot_x * spot_x * HIGHLIGHT_WEIGHT_X + spot_y * spot_y * HIGHLIGHT_WEIGHT_Y
      <= HIGHLIGHT_THRESHOLD
    {
      return [31, 55, 22];
    }
    let (dx, dy) = (col - DISPLAY_CENTER, row - DISPLAY_CENTER);
    let norm = ((dx * dx + dy * dy) as f32 / DISPLAY_RADIUS_SQ as f32).min(1.0);
    [
      (red_c * (1.0 - GRADIENT_FALLOFF_RB * norm)) as i32,
      (green_c * (1.0 - GRADIENT_FALLOFF_G * norm)) as i32,
      (blue_c * (1.0 - GRADIENT_FALLOFF_RB * norm)) as i32,
    ]
  }

  fn assert_matches_reference(eye: &EyeState) {
    let frame = Frame::from_parts(eye, PUPIL_HEIGHT, WIGGLE, RGB);
    let mut out = [0u16; ROW_PIXELS];
    for row in 0..DISPLAY_H {
      frame.render_row(row, &mut out);
      for col in 0..DISPLAY_W {
        let raw = u16::from_be(out[col as usize]);
        let got = [
          i32::from(raw >> 11),
          i32::from((raw >> 5) & 0x3F),
          i32::from(raw & 0x1F),
        ];
        let want = reference_pixel(eye, col, row);
        // The gradient table rounds distance, so allow one step off
        for (channel, (got, want)) in got.iter().zip(want).enumerate() {
          assert!(
            (got - want).abs() <= 1,
            "pixel ({col}, {row}) channel {channel}: got {got}, want {want}"
          );
        }
      }
    }
  }

  #[test]
  fn row_renderer_matches_the_reference_pixel_renderer() {
    let mut eye = EyeState::new(1);
    assert_matches_reference(&eye);

    eye.x = -0.7;
    eye.y = 0.4;
    eye.phase = 12.3;
    eye.blink = 0.6;
    eye.expr_squint = 0.3;
    eye.expr_pupil_scale = 0.7;
    assert_matches_reference(&eye);

    eye.x = 1.0;
    eye.y = -1.0;
    eye.blink = 0.0;
    assert_matches_reference(&eye);
  }

  #[test]
  fn sin_mode_matches_the_reference_pixel_renderer() {
    let mut eye = EyeState::new(2);
    eye.expr_mode = Expression::Sin;
    assert_matches_reference(&eye);

    eye.x = 0.9;
    eye.y = -0.5;
    assert_matches_reference(&eye);
  }
}
