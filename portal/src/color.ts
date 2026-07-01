export type Rgb565 = { red: number; green: number; blue: number };

export function hexToRgb565(hex: string): Rgb565 {
  const red = Math.round((Number.parseInt(hex.slice(1, 3), 16) / 255) * 31);
  const green = Math.round((Number.parseInt(hex.slice(3, 5), 16) / 255) * 63);
  const blue = Math.round((Number.parseInt(hex.slice(5, 7), 16) / 255) * 31);
  return { red, green, blue };
}

export function rgb565ToHex(
  red565: number,
  green565: number,
  blue565: number
): string {
  return (
    '#' +
    Math.round((red565 / 31) * 255)
      .toString(16)
      .padStart(2, '0') +
    Math.round((green565 / 63) * 255)
      .toString(16)
      .padStart(2, '0') +
    Math.round((blue565 / 31) * 255)
      .toString(16)
      .padStart(2, '0')
  );
}
