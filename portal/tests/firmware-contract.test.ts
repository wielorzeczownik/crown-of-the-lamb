import { readFileSync } from 'node:fs';
import { fileURLToPath, URL } from 'node:url';

import { describe, expect, it } from 'vitest';

import { Expression } from '@/renderer';
import { config } from '@/state';

function firmwareSource(path: string): string {
  const url = new URL(`../../${path}`, import.meta.url);
  return readFileSync(fileURLToPath(url), 'utf8');
}

function rustConstant(source: string, name: string): number {
  const pattern = new RegExp(
    String.raw`const ${name}: [a-z0-9]+ = (-?\d+)`
  ).exec(source);
  expect(pattern, `${name} not found in the firmware source`).not.toBeNull();
  return Number(pattern?.[1]);
}

function portalInputs(): Map<string, string> {
  const html = firmwareSource('portal/index.html');
  const inputs = new Map<string, string>();

  for (const [tag] of html.matchAll(/<input[^>]*>/g)) {
    const id = /id="([^"]+)"/.exec(tag);
    if (id) inputs.set(id[1], tag);
  }

  return inputs;
}

const INPUTS = portalInputs();

function sliderRange(id: string): { min: number; max: number } {
  const tag = INPUTS.get(id);
  expect(tag, `no <input id="${id}"> in the portal markup`).toBeDefined();
  const min = /min="(-?\d+)"/.exec(tag ?? '');
  const max = /max="(-?\d+)"/.exec(tag ?? '');
  expect(min, `<input id="${id}"> has no min`).not.toBeNull();
  expect(max, `<input id="${id}"> has no max`).not.toBeNull();
  return { min: Number(min?.[1]), max: Number(max?.[1]) };
}

function portalExpressionCodes(): number[] {
  return Object.values(Expression).filter((value) => typeof value === 'number');
}

describe('expression codes', () => {
  const accepted = Array.from(
    firmwareSource('src/eye.rs').matchAll(/^ +(\d+) => Self::/gm),
    (arm) => Number(arm[1])
  );

  it('parsed the firmware table at all', () => {
    expect(accepted.length).toBeGreaterThan(0);
  });

  it('has an arm for every code the portal can send', () => {
    const codes = portalExpressionCodes();
    expect(codes.length).toBeGreaterThan(0);

    for (const code of codes) {
      expect(
        accepted,
        `firmware rejects expression code ${code}, so the portal would get a 400`
      ).toContain(code);
    }
  });

  it('does not leave a firmware expression unreachable from the portal', () => {
    const codes = portalExpressionCodes();

    for (const code of accepted) {
      expect(
        codes,
        `firmware accepts expression code ${code} but no portal control sends it`
      ).toContain(code);
    }
  });
});

describe('default config', () => {
  const constants = firmwareSource('src/constants.rs');

  it('starts from the same values the firmware boots with', () => {
    const pairs: [keyof typeof config, string][] = [
      ['pupil_length', 'DEFAULT_PUPIL_HEIGHT'],
      ['wiggle_amp', 'DEFAULT_WIGGLE_AMPLITUDE'],
      ['blink_interval', 'DEFAULT_BLINK_INTERVAL'],
      ['eye_red', 'DEFAULT_EYE_RED'],
      ['eye_green', 'DEFAULT_EYE_GREEN'],
      ['eye_blue', 'DEFAULT_EYE_BLUE'],
      ['sound_threshold', 'DEFAULT_SOUND_THRESHOLD'],
      ['sin_rarity', 'DEFAULT_SIN_RARITY'],
      ['eyeroll_rarity', 'DEFAULT_EYEROLL_RARITY'],
      ['startled_rarity', 'DEFAULT_STARTLED_RARITY'],
      ['suspicious_rarity', 'DEFAULT_SUSPICIOUS_RARITY'],
      ['angry_rarity', 'DEFAULT_ANGRY_RARITY'],
    ];

    for (const [field, constant] of pairs) {
      expect(config[field], `${field} vs ${constant}`).toBe(
        rustConstant(constants, constant)
      );
    }
  });
});

const RARITY_CLAMP =
  /^ +([A-Z]+)_RARITY_CFG\.store\(value\.clamp\((\d+), (\d+)\)/;

describe('slider ranges', () => {
  const webConfig = firmwareSource('src/bin/web/config.rs');

  it('cannot offer a value the firmware would clamp away', () => {
    const pairs: [string, number, number][] = [
      [
        'pupil-length',
        rustConstant(webConfig, 'PUPIL_HEIGHT_MIN'),
        rustConstant(webConfig, 'PUPIL_HEIGHT_MAX'),
      ],
      ['wiggle-amp', 0, rustConstant(webConfig, 'WIGGLE_AMP_MAX')],
      [
        'blink-interval',
        rustConstant(webConfig, 'BLINK_INTERVAL_CFG_MIN'),
        rustConstant(webConfig, 'BLINK_INTERVAL_CFG_MAX'),
      ],
      [
        'sound-threshold',
        rustConstant(webConfig, 'SOUND_THRESHOLD_MIN'),
        rustConstant(webConfig, 'SOUND_THRESHOLD_MAX'),
      ],
    ];

    for (const [id, min, max] of pairs) {
      expect(sliderRange(id), `slider ${id}`).toEqual({ min, max });
    }
  });

  it('offers each rarity slider over the percentage range the firmware clamps to', () => {
    const clamps = webConfig
      .split('\n')
      .map((line) => RARITY_CLAMP.exec(line))
      .filter((match) => match !== null)
      .map((match) => ({
        id: `${match[1].toLowerCase()}-rarity`,
        min: Number(match[2]),
        max: Number(match[3]),
      }));

    expect(clamps).toHaveLength(5);

    for (const { id, min, max } of clamps) {
      expect(sliderRange(id), `slider ${id}`).toEqual({ min, max });
    }
  });
});

describe('SSID length', () => {
  it('rejects at the same byte count the firmware stores', () => {
    const storage = firmwareSource('src/storage.rs');
    const wifi = firmwareSource('portal/src/wifi.ts');

    const firmwareMax = /const SSID_MAX_LEN: usize = (\d+)/.exec(storage);
    const portalMax = /const SSID_MAX_BYTES = (\d+)/.exec(wifi);

    expect(firmwareMax).not.toBeNull();
    expect(portalMax).not.toBeNull();
    expect(Number(portalMax?.[1])).toBe(Number(firmwareMax?.[1]));
  });
});
