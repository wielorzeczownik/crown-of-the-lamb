// Rgb565 component maxima and the 8-bit channel max used by the canvas
const RGB565_MAX_R = 31;
const RGB565_MAX_G = 63;
const RGB565_MAX_B = 31;
const RGB888_MAX = 255;

// Display
const DISPLAY_SIZE = 240;
const DISPLAY_CENTER = DISPLAY_SIZE / 2; // 120

// Pupil geometry
const PUPIL_HALF_WIDTH = 12;

// Eye movement range in pixels
const EYE_MOVE_X = 35;
const EYE_MOVE_Y = 20;

// Specular highlight position relative to pupil center
const HIGHLIGHT_OFFSET_X = 22;
const HIGHLIGHT_OFFSET_Y = -26;

// Gradient normalisation:
const DISPLAY_RADIUS_SQ = DISPLAY_CENTER * DISPLAY_CENTER; // 14 400

// Highlight spot colour
const HIGHLIGHT_R = RGB888_MAX;
const HIGHLIGHT_G = Math.round((55 / RGB565_MAX_G) * RGB888_MAX);
const HIGHLIGHT_B = Math.round((22 / RGB565_MAX_B) * RGB888_MAX);
// Highlight ellipse shape
const HIGHLIGHT_WEIGHT_X = 3;
const HIGHLIGHT_WEIGHT_Y = 8;
const HIGHLIGHT_THRESHOLD = 150;

// Expression web codes
export enum Expression {
  Sin = 1,
  EyeRoll = 2,
  Reset = 3,
  Startled = 4,
  Suspicious = 5,
  Angry = 7,
}

export interface EyeRenderState {
  eyeX: number; // -1..1
  eyeY: number; // -1..1
  blink: number; // 0..1
  phase: number;
  pupilRadius: number;
  wiggleAmplitude: number;
  eyeRed: number; // 0-31
  eyeGreen: number; // 0-63
  eyeBlue: number; // 0-31
}

interface RowState {
  isInOpenRegion: boolean;
  halfWidthLeft: number;
  halfWidthRight: number;
  gradientDY2: number;
  highlightDY2Weighted: number;
}

function isInPupil(relativeX: number, rowState: RowState): boolean {
  if (relativeX <= 0) {
    return rowState.halfWidthLeft > 0 && -relativeX <= rowState.halfWidthLeft;
  }
  return rowState.halfWidthRight > 0 && relativeX <= rowState.halfWidthRight;
}

function computeRowState(
  row: number,
  state: EyeRenderState,
  openHalfHeight: number,
  centerY: number,
  highlightY: number
): RowState {
  const isInOpenRegion = Math.abs(row - DISPLAY_CENTER) <= openHalfHeight;
  const gradientDY2 = (row - DISPLAY_CENTER) ** 2;
  const highlightDY = row - highlightY;

  const pupilNormY = (row - centerY) / state.pupilRadius;
  const pupilNormY2 = pupilNormY * pupilNormY;
  const pupilHalfWidth =
    isInOpenRegion && pupilNormY2 < 1
      ? PUPIL_HALF_WIDTH * Math.sqrt(1 - pupilNormY2)
      : 0;

  // Each edge wiggles as a sum of two hand-tuned sine harmonics
  const SECOND_HARMONIC_WEIGHT = 0.5;
  const wiggleLeft =
    Math.sin(state.phase * 0.73 + row * 0.14) * state.wiggleAmplitude +
    Math.sin(state.phase * 2.1 + row * 0.31) *
      (state.wiggleAmplitude * SECOND_HARMONIC_WEIGHT);
  const wiggleRight =
    Math.sin(state.phase * 0.89 + row * 0.16 + 1.1) * state.wiggleAmplitude +
    Math.sin(state.phase * 1.75 + row * 0.28 + 2.3) *
      (state.wiggleAmplitude * SECOND_HARMONIC_WEIGHT);

  return {
    isInOpenRegion,
    halfWidthLeft:
      pupilHalfWidth > 0
        ? Math.max(0, Math.floor(pupilHalfWidth + wiggleLeft))
        : 0,
    halfWidthRight:
      pupilHalfWidth > 0
        ? Math.max(0, Math.floor(pupilHalfWidth + wiggleRight))
        : 0,
    gradientDY2,
    highlightDY2Weighted: highlightDY * highlightDY * HIGHLIGHT_WEIGHT_Y,
  };
}

function applyGradient(
  pixels: Uint8ClampedArray,
  index: number,
  col: number,
  gradientDY2: number,
  redFull: number,
  greenFull: number,
  blueFull: number
): void {
  const gradientDX = col - DISPLAY_CENTER;
  const tLinear = Math.min(
    (gradientDX * gradientDX + gradientDY2) / DISPLAY_RADIUS_SQ,
    1
  );

  // Radius (normalised 0..1) where the two-segment falloff curve bends
  const GRADIENT_BREAK = 0.4;
  const innerT = tLinear / GRADIENT_BREAK;
  const outerT = (tLinear - GRADIENT_BREAK) / (1 - GRADIENT_BREAK);

  // R/B channel
  const factorRB =
    tLinear < GRADIENT_BREAK ? 1 - 0.45 * innerT : 0.55 - 0.5 * outerT;

  // G channel falls off faster for a warmer atmospheric look
  const factorG =
    tLinear < GRADIENT_BREAK ? 1 - 0.72 * innerT : 0.28 - 0.26 * outerT;

  pixels[index] = Math.round(redFull * factorRB);
  pixels[index + 1] = Math.round(greenFull * factorG);
  pixels[index + 2] = Math.round(blueFull * factorRB);
  pixels[index + 3] = RGB888_MAX;
}

function writePixelColor(
  pixels: Uint8ClampedArray,
  pixelIndex: number,
  col: number,
  relativeX: number,
  rowState: RowState,
  highlightX: number,
  redFull: number,
  greenFull: number,
  blueFull: number
): void {
  if (isInPupil(relativeX, rowState)) {
    return;
  }

  const highlightDX = col - highlightX;
  if (
    highlightDX * highlightDX * HIGHLIGHT_WEIGHT_X +
      rowState.highlightDY2Weighted <=
    HIGHLIGHT_THRESHOLD
  ) {
    pixels[pixelIndex] = HIGHLIGHT_R;
    pixels[pixelIndex + 1] = HIGHLIGHT_G;
    pixels[pixelIndex + 2] = HIGHLIGHT_B;
    return;
  }

  applyGradient(
    pixels,
    pixelIndex,
    col,
    rowState.gradientDY2,
    redFull,
    greenFull,
    blueFull
  );
}

export function renderEye(
  context: CanvasRenderingContext2D,
  state: EyeRenderState
): void {
  const centerX = Math.round(DISPLAY_CENTER + state.eyeX * EYE_MOVE_X);
  const centerY = Math.round(DISPLAY_CENTER + state.eyeY * EYE_MOVE_Y);
  const openHalfHeight = Math.round(DISPLAY_CENTER * state.blink);

  // Convert Rgb565 component ranges to 0-255 for canvas
  const redFull = Math.round((state.eyeRed / RGB565_MAX_R) * RGB888_MAX);
  const greenFull = Math.round((state.eyeGreen / RGB565_MAX_G) * RGB888_MAX);
  const blueFull = Math.round((state.eyeBlue / RGB565_MAX_B) * RGB888_MAX);

  const highlightX = centerX + HIGHLIGHT_OFFSET_X;
  const highlightY = centerY + HIGHLIGHT_OFFSET_Y;

  const imageData = context.createImageData(DISPLAY_SIZE, DISPLAY_SIZE);
  const pixels = imageData.data;

  for (let row = 0; row < DISPLAY_SIZE; row++) {
    const rowState = computeRowState(
      row,
      state,
      openHalfHeight,
      centerY,
      highlightY
    );

    for (let col = 0; col < DISPLAY_SIZE; col++) {
      const pixelIndex = (row * DISPLAY_SIZE + col) * 4;
      pixels[pixelIndex + 3] = RGB888_MAX;

      if (rowState.isInOpenRegion) {
        const relativeX = col - centerX;
        writePixelColor(
          pixels,
          pixelIndex,
          col,
          relativeX,
          rowState,
          highlightX,
          redFull,
          greenFull,
          blueFull
        );
      }
    }
  }

  context.putImageData(imageData, 0, 0);
}
