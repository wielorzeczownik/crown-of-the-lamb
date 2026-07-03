import * as api from '@/api';
import { hexToRgb565, rgb565ToHex } from '@/color';
import { createEyeConfigSaver } from '@/eye-config';
import type { LocaleCode } from '@/i18n';
import { applyTranslations, getLocale, setLocale, translate } from '@/i18n';
import { confirm } from '@/modal';
import { initPreview } from '@/preview';
import { Expression } from '@/renderer';
import { config } from '@/state';
import { initSync } from '@/sync';
import { showToast } from '@/toast';
import type { EyeConfig } from '@/types';
import { saveWifi } from '@/wifi';

// DOM references

const tabButtonEye = document.getElementById(
  'tab-btn-eye'
) as HTMLButtonElement;
const tabButtonExpression = document.getElementById(
  'tab-btn-expression'
) as HTMLButtonElement;
const tabButtonSound = document.getElementById(
  'tab-btn-sound'
) as HTMLButtonElement;
const tabButtonNetwork = document.getElementById(
  'tab-btn-network'
) as HTMLButtonElement;
const panelEye = document.getElementById('panel-eye')!;
const panelExpression = document.getElementById('panel-expression')!;
const panelSound = document.getElementById('panel-sound')!;
const panelNetwork = document.getElementById('panel-network')!;
const previewCanvas = document.getElementById(
  'eye-preview'
) as HTMLCanvasElement;
const eyeColorInput = document.getElementById('eye-color') as HTMLInputElement;
const wifiSaveButton = document.getElementById(
  'wifi-save'
)! as HTMLButtonElement;
const wifiSsidInput = document.getElementById('wifi-ssid') as HTMLInputElement;
const localeSelect = document.getElementById(
  'locale-select'
) as HTMLSelectElement;
const configResetButton = document.getElementById(
  'config-reset'
)! as HTMLButtonElement;
const wifiResetButton = document.getElementById(
  'wifi-reset'
)! as HTMLButtonElement;
const expressionResetButton = document.getElementById(
  'expression-reset'
)! as HTMLButtonElement;
const soundResetButton = document.getElementById(
  'sound-reset'
)! as HTMLButtonElement;

// Types

type NumericEyeConfigKey = {
  [K in keyof EyeConfig]: EyeConfig[K] extends number ? K : never;
}[keyof EyeConfig];

type SliderDefine = {
  inputId: string;
  displayId: string;
  configKey: NumericEyeConfigKey;
  suffix?: string;
};

type ResolvedSlider = SliderDefine & {
  inputElement: HTMLInputElement;
  displayElement: Element;
};

// Sliders

// Reflect a range input value as a CSS
function syncFill(input: HTMLInputElement): void {
  const min = Number(input.min) || 0;
  const max = Number(input.max) || 100;
  const pct = ((Number(input.value) - min) / (max - min)) * 100;
  input.style.setProperty('--fill', `${pct}%`);
}

// Eye + Sound sliders
const eyeSliders: SliderDefine[] = [
  {
    inputId: 'pupil-length',
    displayId: 'pupil-length-v',
    configKey: 'pupil_length',
  },
  { inputId: 'wiggle-amp', displayId: 'wiggle-amp-v', configKey: 'wiggle_amp' },
  {
    inputId: 'blink-interval',
    displayId: 'blink-interval-v',
    configKey: 'blink_interval',
  },
  {
    inputId: 'sound-threshold',
    displayId: 'sound-threshold-v',
    configKey: 'sound_threshold',
  },
];

// Expression rarity sliders
const raritySliders: SliderDefine[] = [
  {
    inputId: 'eyeroll-rarity',
    displayId: 'eyeroll-rarity-v',
    configKey: 'eyeroll_rarity',
    suffix: '%',
  },
  {
    inputId: 'startled-rarity',
    displayId: 'startled-rarity-v',
    configKey: 'startled_rarity',
    suffix: '%',
  },
  {
    inputId: 'suspicious-rarity',
    displayId: 'suspicious-rarity-v',
    configKey: 'suspicious_rarity',
    suffix: '%',
  },
  {
    inputId: 'angry-rarity',
    displayId: 'angry-rarity-v',
    configKey: 'angry_rarity',
    suffix: '%',
  },
  {
    inputId: 'sin-rarity',
    displayId: 'sin-rarity-v',
    configKey: 'sin_rarity',
    suffix: '%',
  },
];

const allSliders: SliderDefine[] = [...eyeSliders, ...raritySliders];

const previewContext = previewCanvas.getContext('2d')!;

const resolvedSliders: ResolvedSlider[] = allSliders.map((slider) => ({
  ...slider,
  inputElement: document.getElementById(slider.inputId) as HTMLInputElement,
  displayElement: document.getElementById(slider.displayId)!,
}));

const resolvedRaritySliders = resolvedSliders.filter((slider) =>
  raritySliders.some((rarity) => rarity.configKey === slider.configKey)
);

// Tab navigation

const allTabButtons: HTMLButtonElement[] = [
  tabButtonEye,
  tabButtonExpression,
  tabButtonSound,
  tabButtonNetwork,
];
const allTabPanels: HTMLElement[] = [
  panelEye,
  panelExpression,
  panelSound,
  panelNetwork,
];

const { scheduleEyeSave } = createEyeConfigSaver(eyeColorInput);

const tabState = { activeIndex: 0 };

for (const [index, button] of allTabButtons.entries()) {
  button.addEventListener('click', () => {
    const direction = index > tabState.activeIndex ? 'right' : 'left';
    tabState.activeIndex = index;

    for (const button_ of allTabButtons) {
      button_.classList.remove('active');
      button_.setAttribute('aria-selected', 'false');
    }

    for (const panel of allTabPanels) {
      panel.classList.remove('active');
      delete panel.dataset.dir;
    }

    button.classList.add('active');
    button.setAttribute('aria-selected', 'true');
    button.scrollIntoView({
      behavior: 'smooth',
      block: 'nearest',
      inline: 'nearest',
    });
    const targetPanel = document.getElementById(
      `panel-${button.dataset.tab!}`
    )!;
    targetPanel.dataset.dir = direction;
    targetPanel.classList.add('active');
  });
}

// Eye config inputs

eyeColorInput.addEventListener('input', () => {
  const { red, green, blue } = hexToRgb565(eyeColorInput.value);
  config.eye_red = red;
  config.eye_green = green;
  config.eye_blue = blue;
});
eyeColorInput.addEventListener('change', scheduleEyeSave);

for (const slider of resolvedSliders) {
  syncFill(slider.inputElement);
  slider.inputElement.addEventListener('input', () => {
    const value = Number(slider.inputElement.value);
    slider.displayElement.textContent = String(value) + (slider.suffix ?? '');
    config[slider.configKey] = value;
    syncFill(slider.inputElement);
  });
  slider.inputElement.addEventListener('change', scheduleEyeSave);
}

// WiFi

wifiSaveButton.addEventListener('click', () => {
  void saveWifi(wifiSsidInput);
});

// Resets (config / wifi / expression / sound)

// Default rarity values matching Rust constants
const RARITY_DEFAULTS: Record<string, number> = {
  eyeroll_rarity: 15,
  startled_rarity: 35,
  suspicious_rarity: 28,
  angry_rarity: 55,
  sin_rarity: 1,
};

async function handleConfigReset(): Promise<void> {
  if (!(await confirm(translate('modalConfigResetMsg')))) return;
  try {
    const response = await api.resetConfig();
    if (!response.ok) {
      showToast(false, `${translate('toastError')} ${response.status}`);
      return;
    }
    const loaded = await api.getConfig();
    Object.assign(config, loaded);
    for (const slider of resolvedSliders) {
      const value = String(loaded[slider.configKey]);
      slider.inputElement.value = value;
      slider.displayElement.textContent = value + (slider.suffix ?? '');
      syncFill(slider.inputElement);
    }
    eyeColorInput.value = rgb565ToHex(
      loaded.eye_red,
      loaded.eye_green,
      loaded.eye_blue
    );
    showToast(true, translate('toastSaved'));
  } catch {
    showToast(false, translate('toastNoConnection'));
  }
}
configResetButton.addEventListener('click', () => {
  void handleConfigReset();
});

async function handleWifiReset(): Promise<void> {
  if (!(await confirm(translate('modalWifiResetMsg')))) return;
  try {
    await api.resetWifi();
    showToast(true, translate('toastRestarting'));
  } catch {
    showToast(false, translate('toastNoConnection'));
  }
}
wifiResetButton.addEventListener('click', () => {
  void handleWifiReset();
});

async function handleExpressionReset(): Promise<void> {
  if (!(await confirm(translate('modalExpressionResetMsg')))) return;
  try {
    const response = await api.saveConfig(api.encodeConfig(RARITY_DEFAULTS));
    if (!response.ok) {
      showToast(false, `${translate('toastError')} ${response.status}`);
      return;
    }
    for (const slider of resolvedRaritySliders) {
      const value = RARITY_DEFAULTS[slider.configKey] ?? 0;
      slider.inputElement.value = String(value);
      slider.displayElement.textContent = String(value) + '%';
      config[slider.configKey] = value;
      syncFill(slider.inputElement);
    }
    showToast(true, translate('toastSaved'));
  } catch {
    showToast(false, translate('toastNoConnection'));
  }
}
expressionResetButton.addEventListener('click', () => {
  void handleExpressionReset();
});

const SOUND_THRESHOLD_DEFAULT = 40;

async function handleSoundReset(): Promise<void> {
  if (!(await confirm(translate('modalSoundResetMsg')))) return;
  try {
    await api.saveConfig(
      api.encodeConfig({ sound_threshold: SOUND_THRESHOLD_DEFAULT })
    );
    const slider = resolvedSliders.find(
      (candidate) => candidate.configKey === 'sound_threshold'
    );
    if (slider) {
      slider.inputElement.value = String(SOUND_THRESHOLD_DEFAULT);
      slider.displayElement.textContent = String(SOUND_THRESHOLD_DEFAULT);
      config.sound_threshold = SOUND_THRESHOLD_DEFAULT;
      syncFill(slider.inputElement);
    }
    showToast(true, translate('toastSaved'));
  } catch {
    showToast(false, translate('toastNoConnection'));
  }
}
soundResetButton.addEventListener('click', () => {
  void handleSoundReset();
});

// Locale

localeSelect.addEventListener('change', () =>
  setLocale(localeSelect.value as LocaleCode)
);

// Expression buttons

async function sendExpression(mode: Expression): Promise<void> {
  try {
    await api.sendExpression(mode);
  } catch {
    showToast(false, translate('toastNoConnection'));
  }
}

document.getElementById('expr-eyeroll')!.addEventListener('click', () => {
  void sendExpression(Expression.EyeRoll);
});
document.getElementById('expr-startled')!.addEventListener('click', () => {
  void sendExpression(Expression.Startled);
});
document.getElementById('expr-suspicious')!.addEventListener('click', () => {
  void sendExpression(Expression.Suspicious);
});
document.getElementById('expr-angry')!.addEventListener('click', () => {
  void sendExpression(Expression.Angry);
});
document.getElementById('expr-sin')!.addEventListener('click', () => {
  void sendExpression(Expression.Sin);
});
document.getElementById('expr-reset')!.addEventListener('click', () => {
  void sendExpression(Expression.Reset);
});

// Bootstrap

localeSelect.value = getLocale();
applyTranslations();
initPreview(previewContext);
initSync();

try {
  const loaded = await api.getConfig();
  Object.assign(config, loaded);
  for (const slider of resolvedSliders) {
    const value = String(loaded[slider.configKey]);
    slider.inputElement.value = value;
    slider.displayElement.textContent = value + (slider.suffix ?? '');
    syncFill(slider.inputElement);
  }
  eyeColorInput.value = rgb565ToHex(
    loaded.eye_red,
    loaded.eye_green,
    loaded.eye_blue
  );
} catch {}
