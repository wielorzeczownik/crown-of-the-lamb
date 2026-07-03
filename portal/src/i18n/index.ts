import { en } from '@/i18n/en';
import { pl } from '@/i18n/pl';
import type { Translations } from '@/i18n/translations';

const SUPPORTED = { pl, en } satisfies Record<string, Translations>;

export type LocaleCode = keyof typeof SUPPORTED;

const STORAGE_KEY = 'locale';

type TextBinding = { elementId: string; key: keyof Translations };

// Bindings that set an element attribute
type AttributeBinding = {
  selector: string;
  attribute: string;
  key: keyof Translations;
};

const attributeBindings: AttributeBinding[] = [
  {
    selector: 'meta[name="description"]',
    attribute: 'content',
    key: 'metaDescription',
  },
  { selector: '#eye-preview', attribute: 'aria-label', key: 'ariaEyePreview' },
  { selector: '#locale-select', attribute: 'aria-label', key: 'ariaLanguage' },
];

const textBindings: TextBinding[] = [
  { elementId: 'tab-btn-eye', key: 'tabEye' },
  { elementId: 'tab-btn-expression', key: 'tabExpression' },
  { elementId: 'tab-btn-sound', key: 'tabSound' },
  { elementId: 'tab-btn-network', key: 'tabNetwork' },
  { elementId: 'card-title-eye-color', key: 'cardEyeColor' },
  { elementId: 'label-color', key: 'labelColor' },
  { elementId: 'card-title-pupil', key: 'cardPupil' },
  { elementId: 'label-pupil-length', key: 'labelPupilLength' },
  { elementId: 'label-wiggle', key: 'labelWiggle' },
  { elementId: 'card-title-blink', key: 'cardBlink' },
  { elementId: 'label-blink-interval', key: 'labelBlinkInterval' },
  { elementId: 'card-title-expression', key: 'cardExpression' },
  { elementId: 'expr-eyeroll', key: 'btnEyeRoll' },
  { elementId: 'expr-startled', key: 'btnStartled' },
  { elementId: 'expr-suspicious', key: 'btnSuspicious' },
  { elementId: 'expr-angry', key: 'btnAngry' },
  { elementId: 'expr-sin', key: 'btnSinExpr' },
  { elementId: 'expr-reset', key: 'btnExprReset' },
  { elementId: 'expression-reset', key: 'btnExpressionReset' },
  { elementId: 'label-eyeroll-chance', key: 'labelExprChance' },
  { elementId: 'label-startled-chance', key: 'labelExprChance' },
  { elementId: 'label-suspicious-chance', key: 'labelExprChance' },
  { elementId: 'label-angry-chance', key: 'labelExprChance' },
  { elementId: 'label-sin-chance', key: 'labelExprChance' },
  { elementId: 'card-title-mic', key: 'cardMic' },
  { elementId: 'label-threshold', key: 'labelThreshold' },
  { elementId: 'sound-reset', key: 'btnSoundReset' },
  { elementId: 'card-title-wifi', key: 'cardWifi' },
  { elementId: 'label-wifi-ssid', key: 'labelSsidInput' },
  { elementId: 'wifi-save', key: 'btnWifiSave' },
  { elementId: 'config-reset', key: 'btnConfigReset' },
  { elementId: 'wifi-reset', key: 'btnWifiReset' },
  { elementId: 'modal-confirm', key: 'modalConfirm' },
  { elementId: 'modal-cancel', key: 'modalCancel' },
];

function detectLocale(): LocaleCode {
  const stored = localStorage.getItem(STORAGE_KEY) as LocaleCode | null;
  if (stored && Object.hasOwn(SUPPORTED, stored)) return stored;

  const browserLang = navigator.language
    .slice(0, 2)
    .toLowerCase() as LocaleCode;
  return Object.hasOwn(SUPPORTED, browserLang) ? browserLang : 'en';
}

const localeState = { current: detectLocale() };

export function translate(key: keyof Translations): string {
  return SUPPORTED[localeState.current][key] ?? SUPPORTED.en[key];
}

export function getLocale(): LocaleCode {
  return localeState.current;
}

export function setLocale(locale: LocaleCode): void {
  localeState.current = locale;
  localStorage.setItem(STORAGE_KEY, locale);
  applyTranslations();
}

export function applyTranslations(): void {
  document.documentElement.lang = localeState.current;

  for (const binding of textBindings) {
    const element = document.getElementById(binding.elementId);
    if (element) {
      element.textContent = translate(binding.key);
    }
  }

  for (const binding of attributeBindings) {
    const element = document.querySelector(binding.selector);
    if (element) {
      element.setAttribute(binding.attribute, translate(binding.key));
    }
  }
}
