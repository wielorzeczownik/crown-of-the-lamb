import * as api from '@/api';
import { translate } from '@/i18n';
import { showToast } from '@/toast';

const STORAGE_KEY = 'pending-config';

export function storePending(body: string): void {
  localStorage.setItem(STORAGE_KEY, body);
}

export function clearPending(): void {
  localStorage.removeItem(STORAGE_KEY);
}

async function flushPending(): Promise<void> {
  const body = localStorage.getItem(STORAGE_KEY);
  if (!body) return;
  try {
    const response = await api.saveConfig(body);
    if (response.ok) {
      clearPending();
      showToast(true, translate('toastSaved'));
    }
  } catch {
    // still offline
  }
}

export function initSync(): void {
  globalThis.addEventListener('online', () => {
    void flushPending();
  });
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') void flushPending();
  });

  void flushPending();
}
