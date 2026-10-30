import { afterEach, describe, expect, it, vi } from 'vitest';

import {
  loadDestination,
  onDestinationChanged,
  parseDestination,
  saveDestination,
} from './destination';

afterEach(() => {
  localStorage.clear();
  vi.restoreAllMocks();
});

describe('reject destination', () => {
  it('defaults to the folder', () => {
    expect(loadDestination()).toBe('folder');
  });

  it('remembers the trash', () => {
    saveDestination('trash');

    expect(loadDestination()).toBe('trash');
  });

  it('treats unknown stored values as the folder', () => {
    localStorage.setItem('sito.rejectDestination', 'kosz');

    expect(loadDestination()).toBe('folder');
    expect(parseDestination(null)).toBe('folder');
  });

  it('falls back to the folder when storage is unavailable', () => {
    vi.spyOn(Storage.prototype, 'getItem').mockImplementation(() => {
      throw new Error('blocked');
    });

    expect(loadDestination()).toBe('folder');
  });

  it('reports a destination saved by another window', () => {
    const handler = vi.fn();
    const stop = onDestinationChanged(handler);

    window.dispatchEvent(
      new StorageEvent('storage', { key: 'sito.rejectDestination', newValue: 'trash' }),
    );
    window.dispatchEvent(new StorageEvent('storage', { key: 'sito.timeGap', newValue: '{}' }));
    stop();
    window.dispatchEvent(
      new StorageEvent('storage', { key: 'sito.rejectDestination', newValue: 'folder' }),
    );

    expect(handler.mock.calls).toEqual([['trash']]);
  });
});
