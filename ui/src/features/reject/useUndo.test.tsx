import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { undo, undoState } from './api';
import '../../i18n';
import { useUndo } from './useUndo';

vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  undoState: vi.fn(),
  undo: vi.fn(),
  onTimelineChanged: vi.fn(() => Promise.resolve(() => {})),
}));

const GAP = 12;

describe('useUndo', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(undoState).mockResolvedValue('available');
  });

  it('restores with Ctrl+Z and reports the photo', async () => {
    vi.mocked(undo).mockResolvedValue({
      kind: 'rejection',
      id: 4,
      name: 'IMG_4.jpg',
      restoredName: null,
    } as const);
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(GAP, onUndone));
    await waitFor(() => expect(result.current.undoState).toBe('available'));

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true }));
    });

    await waitFor(() =>
      expect(onUndone).toHaveBeenCalledWith({
        kind: 'rejection',
        id: 4,
        name: 'IMG_4.jpg',
        restoredName: null,
      } as const),
    );
    expect(result.current.toast).toEqual({ text: 'Przywrócono IMG_4.jpg', kind: 'info' });
  });

  it('names the new name of a photo whose own name was taken', async () => {
    vi.mocked(undo).mockResolvedValue({
      kind: 'rejection',
      id: 4,
      name: 'IMG_4.jpg',
      restoredName: 'IMG_4 (2).jpg',
    });
    const { result } = renderHook(() => useUndo(GAP));

    await act(() => result.current.undo());

    expect(result.current.toast).toEqual({
      text: 'Przywrócono IMG_4.jpg jako IMG_4 (2).jpg – nazwa była zajęta.',
      kind: 'info',
    });
  });

  it.each([
    ['occupied', 'Nie można cofnąć: w tym miejscu jest już plik o tej nazwie.'],
    ['missing', 'Nie można cofnąć: plik nie istnieje już w folderze Odrzucone.'],
    ['missingFromTrash', 'Nie można cofnąć: plik nie jest już w Koszu.'],
    [
      'trashUndoUnsupported',
      'Na tym systemie nie można cofnąć przeniesienia do kosza – przywróć zdjęcie z Kosza ręcznie.',
    ],
    ['editUndoFailed', 'Nie udało się cofnąć zmiany serii.'],
    ['internal', 'Nie udało się cofnąć odrzucenia.'],
    ['somethingNew', 'Nie udało się cofnąć odrzucenia.'],
  ])('explains a failed undo (%s)', async (code, text) => {
    vi.mocked(undo).mockRejectedValue({ code });
    const { result } = renderHook(() => useUndo(GAP));

    await act(() => result.current.undo());

    expect(result.current.toast).toEqual({ text, kind: 'error' });
  });

  it.each([
    ['cut', 'Cofnięto podział serii'],
    ['remove', 'Cofnięto wyjęcie IMG_4.jpg z serii'],
    ['join', 'Cofnięto dołączenie IMG_4.jpg do serii'],
    ['ungroup', 'Cofnięto rozdzielenie serii'],
    ['restore', 'Cofnięto przywrócenie automatycznego grupowania'],
  ] as const)('reports an undone %s edit with the gap shown', async (edit, text) => {
    const undone = { kind: 'edit', edit, name: 'IMG_4.jpg', changed: [4, 2] } as const;
    vi.mocked(undo).mockResolvedValue(undone);
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(GAP, onUndone));

    await act(() => result.current.undo());

    expect(undo).toHaveBeenCalledWith(GAP);
    expect(onUndone).toHaveBeenCalledWith(undone);
    expect(result.current.toast).toEqual({ text, kind: 'info' });
  });

  it('does nothing when there is nothing to undo', async () => {
    vi.mocked(undo).mockResolvedValue(null);
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(GAP, onUndone));

    await act(() => result.current.undo());

    expect(onUndone).not.toHaveBeenCalled();
    expect(result.current.toast).toBeNull();
  });

  it('only explains when just rejections into an unrestorable trash are left', async () => {
    vi.mocked(undoState).mockResolvedValue('trashOnly');
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(GAP, onUndone));
    await waitFor(() => expect(result.current.undoState).toBe('trashOnly'));

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true }));
    });

    await waitFor(() =>
      expect(result.current.toast).toEqual({
        text: 'Na tym systemie nie można cofnąć przeniesienia do kosza – przywróć zdjęcie z Kosza ręcznie.',
        kind: 'error',
      }),
    );
    expect(undo).not.toHaveBeenCalled();
    expect(onUndone).not.toHaveBeenCalled();
  });

  it('starts with nothing to undo when the state cannot be read', async () => {
    vi.mocked(undoState).mockRejectedValue(new Error('no backend'));
    const { result } = renderHook(() => useUndo(GAP));

    await waitFor(() => expect(undoState).toHaveBeenCalled());

    expect(result.current.undoState).toBe('none');
  });
});
