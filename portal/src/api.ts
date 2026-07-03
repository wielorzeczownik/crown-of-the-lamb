import type { Expression } from '@/renderer';
import type { EyeConfig } from '@/types';

// The ESP32 picoserve server handles one TCP connection at a time.
// Serialise all API requests so the browser never opens a second connection
// while one is already in flight
const queue = { tail: Promise.resolve() };

async function settle(promise: Promise<unknown>): Promise<void> {
  try {
    await promise;
  } catch {
    // Ignore errors
  }
}

async function runQueued(input: string, init?: RequestInit): Promise<Response> {
  await settle(queue.tail);
  return fetch(input, init);
}

// Low-level transport queues the request so only one is ever in flight
function request(input: string, init?: RequestInit): Promise<Response> {
  const result = runQueued(input, init);
  queue.tail = settle(result);
  return result;
}

// Config

export async function getConfig(): Promise<EyeConfig> {
  const response = await request('/api/config');
  return (await response.json()) as EyeConfig;
}

// Encode a config as a form body
export function encodeConfig(
  fields: Partial<Record<keyof EyeConfig, number>>
): string {
  const parameters = new URLSearchParams();
  for (const [key, value] of Object.entries(fields)) {
    parameters.set(key, String(value));
  }
  return parameters.toString();
}

export function saveConfig(body: string): Promise<Response> {
  return request('/api/config', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body,
  });
}

export function resetConfig(): Promise<Response> {
  return request('/api/config/reset', { method: 'POST' });
}

// Expression

export function sendExpression(mode: Expression): Promise<Response> {
  return request('/api/expression', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ mode: String(mode) }).toString(),
  });
}

// WiFi

export function saveWifi(ssid: string): Promise<Response> {
  return request('/api/wifi', {
    method: 'POST',
    headers: { 'content-type': 'application/x-www-form-urlencoded' },
    body: new URLSearchParams({ ssid }).toString(),
  });
}

export function resetWifi(): Promise<Response> {
  return request('/api/wifi/reset', { method: 'POST' });
}
