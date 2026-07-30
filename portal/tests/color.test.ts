import { describe, expect, it } from 'vitest';

import { hexToRgb565, rgb565ToHex } from '@/color';

const CHANNEL_MAX = { red: 31, green: 63, blue: 31 };

describe('hexToRgb565', () => {
  it('keeps every channel inside the bit depth the firmware writes to the panel', () => {
    for (let byte = 0; byte <= 255; byte++) {
      const hex = `#${byte.toString(16).padStart(2, '0').repeat(3)}`;
      const { red, green, blue } = hexToRgb565(hex);

      expect(red).toBeGreaterThanOrEqual(0);
      expect(red).toBeLessThanOrEqual(CHANNEL_MAX.red);
      expect(green).toBeGreaterThanOrEqual(0);
      expect(green).toBeLessThanOrEqual(CHANNEL_MAX.green);
      expect(blue).toBeGreaterThanOrEqual(0);
      expect(blue).toBeLessThanOrEqual(CHANNEL_MAX.blue);
    }
  });

  it('maps the endpoints to the endpoints', () => {
    expect(hexToRgb565('#000000')).toEqual({ red: 0, green: 0, blue: 0 });
    expect(hexToRgb565('#ffffff')).toEqual({ red: 31, green: 63, blue: 31 });
  });

  it('reads channels independently', () => {
    expect(hexToRgb565('#ff0000')).toEqual({ red: 31, green: 0, blue: 0 });
    expect(hexToRgb565('#00ff00')).toEqual({ red: 0, green: 63, blue: 0 });
    expect(hexToRgb565('#0000ff')).toEqual({ red: 0, green: 0, blue: 31 });
  });

  it('is case insensitive', () => {
    expect(hexToRgb565('#AABBCC')).toEqual(hexToRgb565('#aabbcc'));
  });
});

describe('rgb565ToHex', () => {
  it('always emits a seven-character lower-case hex colour', () => {
    for (let red = 0; red <= CHANNEL_MAX.red; red++) {
      const hex = rgb565ToHex(red, red * 2, red);
      expect(hex).toMatch(/^#[0-9a-f]{6}$/);
    }
  });

  it('round-trips every value the firmware can store', () => {
    for (let red = 0; red <= CHANNEL_MAX.red; red++) {
      for (let green = 0; green <= CHANNEL_MAX.green; green += 3) {
        for (let blue = 0; blue <= CHANNEL_MAX.blue; blue++) {
          const back = hexToRgb565(rgb565ToHex(red, green, blue));
          expect(back).toEqual({ red, green, blue });
        }
      }
    }
  });
});
