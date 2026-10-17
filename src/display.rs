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
// Highlight offset from the pupil centre, ellipse weights, threshold
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
// Lids meet low, the upper one does most of the blinking
const LID_MEET_ROW: f32 = 150.0;
// How far the lids follow the gaze up and down, px at full gaze
const LID_FOLLOW: f32 = 40.0;
// Highlight is a reflection, so it only moves half as far as the pupil
const HIGHLIGHT_PARALLAX: f32 = 0.5;
// Second wiggle harmonic contributes half the first's amplitude
const WIGGLE_SECOND_HARMONIC: f32 = 0.5;
// Sin eye half-width
const SIN_EYE_MAX_DX: i32 = 4;
// Sin slithers like the snake it is. A wave rolls from the head up to the
// tail and grows on the way, so the head with its eye stays put
const SIN_HEAD_Y: i32 = SIN_EYE_Y - 12;
const SIN_BODY_LEN: f32 = 130.0; // head to the tail tip, px
const SLITHER_X: f32 = 4.0; // tail sway per unit of wiggle, px
const SLITHER_Y: f32 = 3.0;
const SLITHER_SPEED: f32 = 1.8;
const SLITHER_WAVE: f32 = 0.1; // about 60 px per wave
const SLITHER_COLS: usize = SIN_BMP_W as usize + 1;
// Room around the Sin for wobbling edges and the soft rim
const SIN_SPAN_PAD: i32 = 6;

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

/// Halves every channel. The mask keeps bits from leaking into the next one
fn soften(px: u16) -> u16 {
  ((u16::from_be(px) >> 1) & 0x7BEF).to_be()
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

/// First open row and one past the last
fn lid_rows(eye: &EyeState) -> (i32, i32) {
  // Sound squint and expression squint, capped at a full squint
  let squint = (eye.extra_close + eye.expr_squint).min(1.0);
  let open = eye.blink * (1.0 - squint * SQUINT_BLINK_FACTOR);
  // Both lids ride along with the gaze, so an eye roll opens up the top
  let follow = eye.y * LID_FOLLOW;
  let top = (LID_MEET_ROW * (1.0 - open) + follow) as i32;
  let bottom = (LID_MEET_ROW + (DISPLAY_H as f32 - LID_MEET_ROW) * open + follow) as i32;
  (top, bottom.max(top))
}

/// Per-frame values, computed once
pub struct Frame {
  center_x: i32,
  center_y: i32,
  lid_top: i32,
  lid_bottom: i32,
  highlight_x: i32,
  highlight_y: i32,
  pupil_height: i32,
  pupil_a: f32,
  wiggle_amp: f32,
  phase: f32,
  sin_mode: bool,
  slither: [f32; SLITHER_COLS], // vertical wave per Sin column
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
    let (lid_top, lid_bottom) = lid_rows(eye);

    let [red, green, blue] = rgb;
    let mut gradient = [BLACK; GRADIENT_LUT_LEN];
    for (idx, entry) in gradient.iter_mut().enumerate() {
      // Hollow-Face radial gradient, bright centre and dark edge
      let dist = (idx << GRADIENT_LUT_SHIFT) as f32;
      let norm = (dist * (1.0 / DISPLAY_RADIUS_SQ as f32)).min(1.0);
      *entry = wire(Rgb565::new(
        (red * (1.0 - GRADIENT_FALLOFF_RB * norm)) as u8,
        (green * (1.0 - GRADIENT_FALLOFF_G * norm)) as u8,
        (blue * (1.0 - GRADIENT_FALLOFF_RB * norm)) as u8,
      ));
    }

    let sin_mode = eye.expr_mode == crate::eye::Expression::Sin;
    let mut slither = [0.0; SLITHER_COLS];
    if sin_mode {
      for (rel_x, wave) in (-SIN_BMP_W / 2..).zip(slither.iter_mut()) {
        *wave = (eye.phase * SLITHER_SPEED - rel_x as f32 * SLITHER_WAVE).sin();
      }
    }

    Self {
      center_x,
      center_y,
      lid_top,
      lid_bottom,
      // Highlight spot, offset right and up from the pupil centre
      highlight_x: DISPLAY_CENTER
        + (eye.x * EYE_MOVE_X * HIGHLIGHT_PARALLAX) as i32
        + HIGHLIGHT_OFFSET_X,
      highlight_y: DISPLAY_CENTER
        + (eye.y * EYE_MOVE_Y * HIGHLIGHT_PARALLAX) as i32
        + HIGHLIGHT_OFFSET_Y,
      pupil_height: (pupil_height as f32 * eye.expr_pupil_scale / eye.stretch) as i32,
      pupil_a: PUPIL_A as f32 * eye.expr_pupil_scale * eye.stretch,
      wiggle_amp,
      phase: eye.phase,
      sin_mode,
      slither,
      eye_color: wire(Rgb565::new(red as u8, green as u8, blue as u8)),
      highlight: wire(HIGHLIGHT),
      gradient,
    }
  }

  /// Draws gradient, highlight, then pupil or Sin, in wire byte order
  pub fn render_row(&self, row: i32, out: &mut [u16]) {
    let out = &mut out[..ROW_PIXELS];
    // Half-open range, so a shut lid really is shut
    if !(self.lid_top..self.lid_bottom).contains(&row) {
      out.fill(0);
      return;
    }

    let delta_y = row - DISPLAY_CENTER;
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
      self.paint_sin(out, row, rel_y);
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

    let (w_left, w_right) = self.edge_wiggle(row);
    let left_edge = (pupil_half_w + w_left).max(0.0) as i32;
    let right_edge = (pupil_half_w + w_right).max(0.0) as i32;
    if left_edge > 0 {
      fill_span(out, self.center_x - left_edge, self.center_x, BLACK);
    }
    if right_edge > 0 {
      fill_span(out, self.center_x + 1, self.center_x + right_edge, BLACK);
    }
  }

  /// How far the left and right edge bulge out on this row, px
  fn edge_wiggle(&self, row: i32) -> (f32, f32) {
    // Two independent sine signals per side, giving asymmetric organic wiggle
    let (phase, amp, row_f) = (self.phase, self.wiggle_amp, row as f32);
    let w_left = (phase * 0.73 + row_f * 0.14).sin() * amp
      + (phase * 2.1 + row_f * 0.31).sin() * (amp * WIGGLE_SECOND_HARMONIC);
    let w_right = (phase * 0.89 + row_f * 0.16 + 1.1).sin() * amp
      + (phase * 1.75 + row_f * 0.28 + 2.3).sin() * (amp * WIGGLE_SECOND_HARMONIC);
    (w_left, w_right)
  }

  fn paint_sin(&self, out: &mut [u16], row: i32, rel_y: i32) {
    let reach = ((SIN_HEAD_Y - rel_y) as f32 / SIN_BODY_LEN).clamp(0.0, 1.0);
    let lift = reach * self.wiggle_amp * SLITHER_Y;
    let half_h = SIN_BMP_H / 2 + lift as i32 + 1;
    if !(-half_h..half_h).contains(&rel_y) {
      return;
    }
    let (w_left, w_right) = self.edge_wiggle(row);
    let sway = (self.phase * SLITHER_SPEED + rel_y as f32 * SLITHER_WAVE).sin()
      * reach
      * self.wiggle_amp
      * SLITHER_X;
    let origin = self.center_x + sway as i32;
    let half_w = SIN_BMP_W / 2;

    // Every run of ink gets wobbly edges like the pupil, never thinner than a pixel
    let mut painted = [0_u32; ROW_PIXELS.div_ceil(32)];
    let mut run_start = None;
    for (rel_x, &wave) in (-half_w..=half_w).zip(self.slither.iter()) {
      let inked = rel_x < half_w && in_sin_bitmap(rel_x, rel_y - (wave * lift) as i32);
      match (inked, run_start) {
        (true, None) => run_start = Some(rel_x),
        (false, Some(start)) => {
          run_start = None;
          let from = origin + start - w_left as i32;
          let to = (origin + rel_x - 1 + w_right as i32).max(from);
          for col in from.max(0)..=to.min(DISPLAY_W - 1) {
            painted[col as usize / 32] |= 1 << (col % 32);
          }
        }
        _ => {}
      }
    }

    // Ink goes black, the pixel right next to it half dark for a soft rim
    let is_ink = |col: i32| {
      (0..DISPLAY_W).contains(&col) && painted[col as usize / 32] & (1 << (col % 32)) != 0
    };
    let span =
      (origin - half_w - SIN_SPAN_PAD).max(0)..(origin + half_w + SIN_SPAN_PAD).min(DISPLAY_W);
    for col in span {
      let px = &mut out[col as usize];
      if is_ink(col) {
        *px = BLACK;
      } else if is_ink(col - 1) || is_ink(col + 1) {
        *px = soften(*px);
      }
    }

    for rel_x in SIN_EYE_X - SIN_EYE_MAX_DX..=SIN_EYE_X + SIN_EYE_MAX_DX {
      if in_sin_eye(rel_x, rel_y) {
        let color = if in_sin_eye_slit(rel_x, rel_y) {
          BLACK
        } else {
          self.eye_color
        };
        let col = origin + rel_x;
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
    HIGHLIGHT_PARALLAX, HIGHLIGHT_THRESHOLD, HIGHLIGHT_WEIGHT_X, HIGHLIGHT_WEIGHT_Y, PUPIL_A,
    ROW_PIXELS, WIGGLE_SECOND_HARMONIC, in_sin_bitmap, in_sin_eye, in_sin_eye_slit, lid_rows,
  };
  use crate::eye::{Expression, EyeState};

  const PUPIL_HEIGHT: i32 = 70;
  const WIGGLE: f32 = 1.4;
  const RGB: [f32; 3] = [26.0, 40.0, 9.0];

  /// The original per-pixel float renderer, kept as the visual reference
  fn reference_pixel(eye: &EyeState, wiggle: f32, col: i32, row: i32) -> [i32; 3] {
    let center_x = DISPLAY_CENTER + (eye.x * EYE_MOVE_X) as i32;
    let center_y = DISPLAY_CENTER + (eye.y * EYE_MOVE_Y) as i32;
    let (lid_top, lid_bottom) = lid_rows(eye);
    if !(lid_top..lid_bottom).contains(&row) {
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
      if in_sin_bitmap(rel_x - 1, rel_y) || in_sin_bitmap(rel_x + 1, rel_y) {
        return reference_backdrop(eye, col, row).map(|channel| channel / 2);
      }
    } else {
      let norm_t =
        rel_y as f32 / (PUPIL_HEIGHT as f32 * eye.expr_pupil_scale / eye.stretch) as i32 as f32;
      let norm_t_sq = norm_t * norm_t;
      let half_w = if norm_t_sq >= 1.0 {
        0.0
      } else {
        PUPIL_A as f32 * eye.expr_pupil_scale * eye.stretch * (1.0 - norm_t_sq).sqrt()
      };
      let (phase, row_f) = (eye.phase, row as f32);
      let w_left = (phase * 0.73 + row_f * 0.14).sin() * wiggle
        + (phase * 2.1 + row_f * 0.31).sin() * (wiggle * WIGGLE_SECOND_HARMONIC);
      let w_right = (phase * 0.89 + row_f * 0.16 + 1.1).sin() * wiggle
        + (phase * 1.75 + row_f * 0.28 + 2.3).sin() * (wiggle * WIGGLE_SECOND_HARMONIC);
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

    reference_backdrop(eye, col, row)
  }

  /// Highlight or gradient, whatever sits under the pupil and the Sin
  fn reference_backdrop(eye: &EyeState, col: i32, row: i32) -> [i32; 3] {
    let [red_c, green_c, blue_c] = RGB;
    let spot_x = col
      - (DISPLAY_CENTER + (eye.x * EYE_MOVE_X * HIGHLIGHT_PARALLAX) as i32 + HIGHLIGHT_OFFSET_X);
    let spot_y = row
      - (DISPLAY_CENTER + (eye.y * EYE_MOVE_Y * HIGHLIGHT_PARALLAX) as i32 + HIGHLIGHT_OFFSET_Y);
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

  fn assert_matches_reference(eye: &EyeState, wiggle: f32) {
    let frame = Frame::from_parts(eye, PUPIL_HEIGHT, wiggle, RGB);
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
        let want = reference_pixel(eye, wiggle, col, row);
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
    // Skip the boot greeting, a shut eye would compare black to black
    eye.blink = 1.0;
    eye.expr_squint = 0.0;
    assert_matches_reference(&eye, WIGGLE);

    eye.x = -0.7;
    eye.y = 0.4;
    eye.phase = 12.3;
    eye.blink = 0.6;
    eye.expr_squint = 0.3;
    eye.expr_pupil_scale = 0.7;
    eye.stretch = 1.3;
    assert_matches_reference(&eye, WIGGLE);

    eye.x = 1.0;
    eye.y = -1.0;
    eye.blink = 0.0;
    assert_matches_reference(&eye, WIGGLE);
  }

  #[test]
  fn a_shut_eye_leaves_no_strip() {
    let eye = EyeState::new(3);
    assert!(eye.blink < f32::EPSILON, "boots with the lid shut");
    let frame = Frame::from_parts(&eye, PUPIL_HEIGHT, WIGGLE, RGB);
    let mut out = [1_u16; ROW_PIXELS];
    for row in 0..DISPLAY_H {
      frame.render_row(row, &mut out);
      assert!(out.iter().all(|&px| px == 0), "row {row} lit on a shut eye");
    }
  }

  #[test]
  fn the_upper_lid_does_most_of_the_blinking() {
    let mut eye = EyeState::new(4);
    eye.blink = 0.5;
    eye.expr_squint = 0.0;
    let (top, bottom) = lid_rows(&eye);
    assert!(
      top > DISPLAY_H - bottom,
      "upper lid at {top} should cover more than the lower one at {bottom}"
    );

    eye.blink = 1.0;
    eye.y = 1.0;
    assert!(
      lid_rows(&eye).0 > 0,
      "looking down should drag the upper lid along"
    );
  }

  #[test]
  fn sin_mode_matches_the_reference_pixel_renderer() {
    let mut eye = EyeState::new(2);
    eye.blink = 1.0;
    eye.expr_squint = 0.0;
    eye.expr_mode = Expression::Sin;
    // Without wiggle the Sin holds still, so the per-pixel reference can follow it
    assert_matches_reference(&eye, 0.0);

    eye.x = 0.9;
    eye.y = -0.5;
    assert_matches_reference(&eye, 0.0);
  }

  #[test]
  fn sin_slithers_but_its_eye_stays_put() {
    let mut eye = EyeState::new(7);
    eye.blink = 1.0;
    eye.expr_squint = 0.0;
    eye.expr_mode = Expression::Sin;
    // One hash for the whole picture, one for where the eye colour lands
    let snapshot = |eye: &EyeState| {
      let frame = Frame::from_parts(eye, PUPIL_HEIGHT, WIGGLE, RGB);
      let mut out = [0_u16; ROW_PIXELS];
      let (mut picture, mut eye_spot) = (0_u64, 0_u64);
      for row in 0..DISPLAY_H {
        frame.render_row(row, &mut out);
        for (col, &px) in out.iter().enumerate() {
          picture = picture.wrapping_mul(31).wrapping_add(u64::from(px));
          if row > DISPLAY_CENTER && px == frame.eye_color {
            eye_spot = eye_spot
              .wrapping_mul(31)
              .wrapping_add((row as u64) << 8 | col as u64);
          }
        }
      }
      (picture, eye_spot)
    };
    let before = snapshot(&eye);
    eye.phase += 1.0;
    let after = snapshot(&eye);
    assert!(before.0 != after.0, "the Sin should move between frames");
    assert!(before.1 != 0, "the eye never showed up");
    assert!(before.1 == after.1, "the head should hold its eye still");
  }
}
