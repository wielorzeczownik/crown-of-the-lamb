import { afterEach, describe, expect, it, vi } from 'vitest';

import * as api from '@/api';
import { Expression } from '@/renderer';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('encodeConfig', () => {
  it('encodes only the fields it was handed', () => {
    const body = api.encodeConfig({ eye_red: 26, eye_green: 5 });
    expect(new URLSearchParams(body)).toEqual(
      new URLSearchParams({ eye_red: '26', eye_green: '5' })
    );
  });

  it('percent-encodes rather than emitting a raw separator', () => {
    expect(api.encodeConfig({})).toBe('');
    expect(api.encodeConfig({ pupil_length: -1 })).toBe('pupil_length=-1');
  });
});

describe('request serialisation', () => {
  it('never has two requests in flight at once', async () => {
    let inFlight = 0;
    let peak = 0;

    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        inFlight++;
        peak = Math.max(peak, inFlight);
        await new Promise((resolve) => setTimeout(resolve, 1));
        inFlight--;
        return new Response(undefined, { status: 204 });
      })
    );

    await Promise.all([
      api.saveConfig('eye_red=1'),
      api.sendExpression(Expression.Angry),
      api.resetConfig(),
      api.saveWifi('crown'),
      api.resetWifi(),
    ]);

    expect(peak).toBe(1);
  });

  it('keeps draining the queue after a request rejects', async () => {
    const fetchMock = vi
      .fn()
      .mockRejectedValueOnce(new Error('connection reset'))
      .mockResolvedValue(new Response(undefined, { status: 204 }));
    vi.stubGlobal('fetch', fetchMock);

    const failing = api.saveConfig('eye_red=1');
    const following = api.resetConfig();

    await expect(failing).rejects.toThrow('connection reset');
    await expect(following).resolves.toMatchObject({ status: 204 });
    expect(fetchMock).toHaveBeenCalledTimes(2);
  });
});

describe('request bodies', () => {
  it('sends form-encoded bodies, which is all the firmware Form extractor parses', async () => {
    const fetchMock = vi
      .fn()
      .mockResolvedValue(new Response(undefined, { status: 204 }));
    vi.stubGlobal('fetch', fetchMock);

    await api.saveWifi('Crown of the Lamb');

    const [url, init] = fetchMock.mock.calls[0] as [string, RequestInit];
    expect(url).toBe('/api/wifi');
    expect(init.method).toBe('POST');
    expect(init.headers).toMatchObject({
      'content-type': 'application/x-www-form-urlencoded',
    });
    expect(new URLSearchParams(init.body as string).get('ssid')).toBe(
      'Crown of the Lamb'
    );
  });
});
