import { describe, expect, it, vi } from 'vitest';

import type { ViewerPhoto, ViewerRequest } from './api';
import { ViewerPageCache, type LoadPage } from './viewerPageCache';

function request(id: number, changes: Partial<ViewerRequest> = {}): ViewerRequest {
  return { id, scanId: 1, mode: 'all', gap: 10, ...changes };
}

/** A loader that answers every request with `null`, counting the calls. */
function loader() {
  return vi.fn<LoadPage>(() => Promise.resolve<ViewerPhoto | null>(null));
}

describe('ViewerPageCache', () => {
  it('loads each page of a view once', async () => {
    const load = loader();
    const cache = new ViewerPageCache(load);

    await cache.get(request(1), 0);
    await cache.get(request(1), 0);
    await cache.get(request(2), 0);

    expect(load).toHaveBeenCalledTimes(2);
  });

  it('loads again for another mode, gap, scan or timeline revision', async () => {
    const load = loader();
    const cache = new ViewerPageCache(load);

    await cache.get(request(1), 0);
    await cache.get(request(1, { mode: 'series' }), 0);
    await cache.get(request(1, { mode: 'series', gap: 5 }), 0);
    await cache.get(request(1, { mode: 'series', gap: 5, scanId: 2 }), 0);
    await cache.get(request(1, { mode: 'series', gap: 5, scanId: 2 }), 1);

    expect(load).toHaveBeenCalledTimes(5);
  });

  it('tries a failed page again', async () => {
    const load = vi
      .fn<LoadPage>()
      .mockRejectedValueOnce(new Error('busy'))
      .mockResolvedValueOnce(null);
    const cache = new ViewerPageCache(load);

    await expect(cache.get(request(1), 0)).rejects.toThrow('busy');
    await expect(cache.get(request(1), 0)).resolves.toBeNull();

    expect(load).toHaveBeenCalledTimes(2);
  });
});
