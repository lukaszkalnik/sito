import { invoke } from '@tauri-apps/api/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { undo, undoState } from './api';

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

describe('undo', () => {
  beforeEach(() => vi.clearAllMocks());

  it('asks for the gap shown and reads a restored photo', async () => {
    vi.mocked(invoke).mockResolvedValue({
      kind: 'rejection',
      id: 4,
      name: 'IMG_4.jpg',
      restoredName: null,
    });

    await expect(undo(12)).resolves.toEqual({
      kind: 'rejection',
      id: 4,
      name: 'IMG_4.jpg',
      restoredName: null,
    });
    expect(invoke).toHaveBeenCalledWith('undo', { maxGapSeconds: 12 });
  });

  it('reads an undone edit', async () => {
    const undone = { kind: 'edit', edit: 'join', name: 'IMG_4.jpg', changed: [4, 2] };
    vi.mocked(invoke).mockResolvedValue(undone);

    await expect(undo(10)).resolves.toEqual(undone);
  });

  it('reads nothing undone', async () => {
    vi.mocked(invoke).mockResolvedValue(null);

    await expect(undo(10)).resolves.toBeNull();
  });

  it.each([
    { kind: 'edit', edit: 'shuffle', name: 'a.jpg', changed: [] },
    { kind: 'edit', edit: 'cut', name: 'a.jpg' },
    { kind: 'something', name: 'a.jpg' },
    { kind: 'rejection' },
  ])('refuses an unknown answer (%o)', async (answer) => {
    vi.mocked(invoke).mockResolvedValue(answer);

    await expect(undo(10)).rejects.toThrow();
  });
});
