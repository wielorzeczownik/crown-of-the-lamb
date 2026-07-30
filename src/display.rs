use embedded_graphics::{
  draw_target::DrawTarget,
  geometry::{Point, Size},
  pixelcolor::Rgb565,
  primitives::Rectangle,
};
#[cfg_attr(not(target_arch = "xtensa"), allow(unused_imports))]
use micromath::F32Ext;

use crate::eye::EyeState;

// Display geometry
const DISPLAY_W: i32 = 240;
const DISPLAY_H: i32 = 240;
const DISPLAY_CENTER: i32 = DISPLAY_W / 2;
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
// Radial gradient falloff from centre (1.0) to display edge
const GRADIENT_FALLOFF_RB: f32 = 0.46; // red & blue
const GRADIENT_FALLOFF_G: f32 = 0.80; // green falls off faster (warmer edge)
// Extra eyelid close from a full squint, on top of the blink amount
const SQUINT_BLINK_FACTOR: f32 = 0.65;
// Second wiggle harmonic contributes half the first's amplitude
const WIGGLE_SECOND_HARMONIC: f32 = 0.5;

const BLACK: Rgb565 = Rgb565::new(0, 0, 0);
const HIGHLIGHT: Rgb565 = Rgb565::new(31, 55, 22);

use crate::sin_bitmap::{SIN_BMP, SIN_BMP_H, SIN_BMP_ROW, SIN_BMP_W, SIN_EYE_X, SIN_EYE_Y};

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

const DISPLAY_RADIUS_SQ: f32 = (DISPLAY_CENTER * DISPLAY_CENTER) as f32; // for radial gradient

pub fn draw_eye<D: DrawTarget<Color = Rgb565>>(display: &mut D, eye: &EyeState) {
  let center_x = DISPLAY_CENTER + (eye.x * EYE_MOVE_X) as i32;
  let center_y = DISPLAY_CENTER + (eye.y * EYE_MOVE_Y) as i32;
  let phase = eye.phase;
  let sin_mode = eye.expr_mode == crate::eye::Expression::Sin;

  // Combine sound squint and expression squint,
  // both smoothly lerped, capped so they can't exceed full-squint
  let squint = (eye.extra_close + eye.expr_squint).min(1.0);
  let effective_blink = eye.blink * (1.0 - squint * SQUINT_BLINK_FACTOR);
  let open_half_h = (DISPLAY_CENTER as f32 * effective_blink) as i32;

  // Highlight spot, offset right and up from the pupil centre
  let highlight_x = center_x + HIGHLIGHT_OFFSET_X;
  let highlight_y = center_y + HIGHLIGHT_OFFSET_Y;

  let pupil_scale = eye.expr_pupil_scale;
  let pupil_height = (crate::config::pupil_height() as f32 * pupil_scale) as i32;
  let pupil_a = PUPIL_A as f32 * pupil_scale;
  let wiggle_amp = crate::config::wiggle_amplitude();
  let red_center = crate::config::eye_color_red();
  let green_center = crate::config::eye_color_green();
  let blue_center = crate::config::eye_color_blue();

  let full_rect = Rectangle::new(Point::zero(), Size::new(DISPLAY_W as u32, DISPLAY_H as u32));

  let _ = display.fill_contiguous(
    &full_rect,
    (0..DISPLAY_H).flat_map(move |row| {
      let in_open_area = (row - DISPLAY_CENTER).abs() <= open_half_h;

      let delta_y_sq = (row - DISPLAY_CENTER) * (row - DISPLAY_CENTER);
      let row_from_spot = row - highlight_y;
      let row_from_spot_sq8 = row_from_spot * row_from_spot * HIGHLIGHT_WEIGHT_Y;

      // Compute pupil half-width for this row.
      let pupil_half_w: f32 = if in_open_area {
        let norm_t = (row - center_y) as f32 / pupil_height as f32;
        let norm_t_sq = norm_t * norm_t;
        if norm_t_sq >= 1.0 {
          0.0
        } else {
          pupil_a * (1.0_f32 - norm_t_sq).sqrt()
        }
      } else {
        0.0
      };

      // Two independent sine signals per side, giving asymmetric organic wiggle
      let row_f = row as f32;
      let w_left = (phase * 0.73 + row_f * 0.14).sin() * wiggle_amp
        + (phase * 2.1 + row_f * 0.31).sin() * (wiggle_amp * WIGGLE_SECOND_HARMONIC);
      let w_right = (phase * 0.89 + row_f * 0.16 + 1.1).sin() * wiggle_amp
        + (phase * 1.75 + row_f * 0.28 + 2.3).sin() * (wiggle_amp * WIGGLE_SECOND_HARMONIC);

      // Wiggle only active inside the pupil ellipse
      let left_pupil_edge = if pupil_half_w > 0.0 {
        (pupil_half_w + w_left).max(0.0) as i32
      } else {
        0
      };
      let right_pupil_edge = if pupil_half_w > 0.0 {
        (pupil_half_w + w_right).max(0.0) as i32
      } else {
        0
      };

      (0..DISPLAY_W).map(move |col| {
        if !in_open_area {
          return BLACK;
        }

        let rel_x = col - center_x;
        let rel_y = row - center_y;

        if sin_mode {
          if in_sin_eye(rel_x, rel_y) {
            return if in_sin_eye_slit(rel_x, rel_y) {
              BLACK
            } else {
              Rgb565::new(red_center as u8, green_center as u8, blue_center as u8)
            };
          }
          if in_sin_bitmap(rel_x, rel_y) {
            return BLACK;
          }
        } else {
          let delta_x = rel_x;
          if delta_x <= 0 && left_pupil_edge > 0 && (-delta_x) <= left_pupil_edge {
            return BLACK;
          }
          if delta_x > 0 && right_pupil_edge > 0 && delta_x <= right_pupil_edge {
            return BLACK;
          }
        }

        // Hollow-Face radial gradient: bright centre, dark edge.
        let col_offset = col - DISPLAY_CENTER;
        let dist = (col_offset * col_offset + delta_y_sq) as f32;
        let norm = (dist * (1.0 / DISPLAY_RADIUS_SQ)).min(1.0);
        let red = (red_center * (1.0 - GRADIENT_FALLOFF_RB * norm)) as u8;
        let green = (green_center * (1.0 - GRADIENT_FALLOFF_G * norm)) as u8;
        let blue = (blue_center * (1.0 - GRADIENT_FALLOFF_RB * norm)) as u8;

        let col_from_spot = col - highlight_x;
        if col_from_spot * col_from_spot * HIGHLIGHT_WEIGHT_X + row_from_spot_sq8
          <= HIGHLIGHT_THRESHOLD
        {
          return HIGHLIGHT;
        }

        Rgb565::new(red, green, blue)
      })
    }),
  );
}
