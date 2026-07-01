import { renderEye } from '@/renderer';
import { config } from '@/state';

// Wiggle phase advanced per animation frame
const PHASE_STEP = 0.038;
// Eyelid open/close progress per frame while a blink is in motion
const BLINK_STEP = 0.18;
// Frames to wait before the next blink
const BLINK_DELAY_MIN = 180;
const BLINK_DELAY_SPREAD = 300;
// Eyelid open
const BLINK_OPEN = 1;
const BLINK_CLOSED = 0;
// wiggle_amp is stored as a percentage
const WIGGLE_AMP_PERCENT = 100;

const preview = {
  phase: 0,
  blink: BLINK_OPEN,
  blinkDirection: -1,
  blinkTimer: BLINK_DELAY_MIN,
};

export function initPreview(context: CanvasRenderingContext2D): void {
  function loop(): void {
    preview.phase += PHASE_STEP;

    preview.blinkTimer--;
    if (preview.blinkTimer <= 0) {
      preview.blink = Math.max(
        BLINK_CLOSED,
        Math.min(
          BLINK_OPEN,
          preview.blink + preview.blinkDirection * BLINK_STEP
        )
      );
      if (preview.blink <= BLINK_CLOSED) preview.blinkDirection = 1;

      if (preview.blink >= BLINK_OPEN) {
        preview.blinkDirection = -1;
        preview.blinkTimer = Math.trunc(
          BLINK_DELAY_MIN + Math.random() * BLINK_DELAY_SPREAD
        );
      }
    }

    renderEye(context, {
      eyeX: 0,
      eyeY: 0,
      blink: preview.blink,
      phase: preview.phase,
      pupilRadius: config.pupil_length,
      wiggleAmplitude: config.wiggle_amp / WIGGLE_AMP_PERCENT,
      eyeRed: config.eye_red,
      eyeGreen: config.eye_green,
      eyeBlue: config.eye_blue,
    });

    requestAnimationFrame(loop);
  }

  requestAnimationFrame(loop);
}
