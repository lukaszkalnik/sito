import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { undoState } from './api';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

describe('undoState', () => {
  beforeEach(() => vi.clearAllMocks());

  it.each(['available', 'none', 'trashOnly'] as const)('reads %s', async (kind) => {
    vi.mocked(invoke).mockResolvedValue({ kind });

    await expect(undoState()).resolves.toBe(kind);
  });

  it.each([{ kind: 'somethingNew' }, { canUndo: true }, null])(
    'treats an unknown answer as nothing to undo (%o)',
    async (answer) => {
      vi.mocked(invoke).mockResolvedValue(answer);

      await expect(undoState()).resolves.toBe('none');
    },
  );
});
