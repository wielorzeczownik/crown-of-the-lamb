import * as api from '@/api';
import { translate } from '@/i18n';
import { showToast } from '@/toast';

// Firmware stores the SSID in a fixed 32-byte string, so an over-long name is
// silently dropped to an empty string on the device. Reject it here in bytes
// (not chars) — multibyte characters make a 32-char name exceed 32 bytes.
const SSID_MAX_BYTES = 32;

export async function saveWifi(ssidInput: HTMLInputElement): Promise<void> {
  const ssid = ssidInput.value.trim();
  if (!ssid) {
    showToast(false, translate('toastEmptySsid'));
    return;
  }
  if (new TextEncoder().encode(ssid).length > SSID_MAX_BYTES) {
    showToast(false, translate('toastSsidTooLong'));
    return;
  }

  showToast(true, translate('toastSaving'));
  try {
    const response = await api.saveWifi(ssid);
    if (!response.ok) {
      showToast(false, `${translate('toastError')} ${response.status}`);
      return;
    }
    showToast(true, translate('toastRestarting'));
  } catch {
    showToast(false, translate('toastNoConnection'));
  }
}
