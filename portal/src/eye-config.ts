import * as api from '@/api';
import { hexToRgb565 } from '@/color';
import { translate } from '@/i18n';
import { config } from '@/state';
import { clearPending, storePending } from '@/sync';
import { showToast } from '@/toast';

export function createEyeConfigSaver(colorInput: HTMLInputElement): {
  scheduleEyeSave: () => void;
} {
  let eyeSaveTimer: ReturnType<typeof setTimeout> | undefined;

  async function saveEyeConfig(): Promise<void> {
    const { red, green, blue } = hexToRgb565(colorInput.value);
    const body = api.encodeConfig({
      eye_red: red,
      eye_green: green,
      eye_blue: blue,
      pupil_length: config.pupil_length,
      wiggle_amp: config.wiggle_amp,
      blink_interval: config.blink_interval,
      sound_threshold: config.sound_threshold,
      sin_rarity: config.sin_rarity,
      eyeroll_rarity: config.eyeroll_rarity,
      startled_rarity: config.startled_rarity,
      suspicious_rarity: config.suspicious_rarity,
      angry_rarity: config.angry_rarity,
    });

    storePending(body);

    try {
      const response = await api.saveConfig(body);
      if (response.ok) {
        clearPending();
        showToast(true, translate('toastSaved'));
      } else {
        showToast(false, `${translate('toastError')} ${response.status}`);
      }
    } catch {
      showToast(false, translate('toastQueued'));
    }
  }

  function scheduleEyeSave(): void {
    if (eyeSaveTimer) clearTimeout(eyeSaveTimer);
    eyeSaveTimer = setTimeout(() => {
      void saveEyeConfig();
    }, 120);
  }

  return { scheduleEyeSave };
}
