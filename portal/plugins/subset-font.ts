import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { fileURLToPath, URL } from 'node:url';

import subsetFont from 'subset-font';
import type { Plugin } from 'vite';

const fromRoot = (path: string): string =>
  fileURLToPath(new URL(`../${path}`, import.meta.url));

const SOURCE_FONT = fromRoot('src/style/fonts/Piazzolla[opsz,wght].ttf');
const OUTPUT_DIR = fromRoot('src/style/fonts');
const TEXT_SOURCES = ['src/i18n/en.ts', 'src/i18n/pl.ts', 'index.html'];
const POLISH_LETTERS = 'ąćęłńóśźżĄĆĘŁŃÓŚŹŻ';
const WEIGHTS = [600, 700];
const OPTICAL_SIZE = 14;

function collectGlyphs(): string {
  const chars = new Set<string>();
  for (const source of TEXT_SOURCES) {
    for (const ch of readFileSync(fromRoot(source), 'utf8')) chars.add(ch);
  }
  for (let code = 0x20; code <= 0x7e; code++) {
    chars.add(String.fromCodePoint(code));
  }
  for (const ch of POLISH_LETTERS) chars.add(ch);
  return [...chars].filter((ch) => ch.codePointAt(0)! >= 0x20).join('');
}

export function subsetPiazzolla(): Plugin {
  return {
    name: 'subset-font',
    async buildStart() {
      const text = collectGlyphs();
      const source = readFileSync(SOURCE_FONT);
      mkdirSync(OUTPUT_DIR, { recursive: true });
      for (const wght of WEIGHTS) {
        const woff2 = await subsetFont(source, text, {
          targetFormat: 'woff2',
          variationAxes: { wght, opsz: OPTICAL_SIZE },
        });
        writeFileSync(`${OUTPUT_DIR}/piazzolla-${wght}.woff2`, woff2);
      }
    },
  };
}
