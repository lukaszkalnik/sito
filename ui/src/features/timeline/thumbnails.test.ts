import { describe, expect, it, vi } from 'vitest';

import { thumbnailUrl } from './thumbnails';

vi.mock('@tauri-apps/api/core', () => ({
  convertFileSrc: (path: string, protocol: string) => `http://${protocol}.localhost/${path}`,
}));

describe('thumbnailUrl', () => {
  it('points at the thumb protocol and carries the scan id', () => {
    expect(thumbnailUrl(1700000000000, 42)).toBe('http://thumb.localhost/42?scan=1700000000000');
  });
});
