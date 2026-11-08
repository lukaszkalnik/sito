import { act, renderHook, waitFor } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { undoLast, undoState } from './api';
import '../../i18n';
import { useUndo } from './useUndo';

vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  undoState: vi.fn(),
  undoLast: vi.fn(),
  onTimelineChanged: vi.fn(() => Promise.resolve(() => {})),
}));

describe('useUndo', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(undoState).mockResolvedValue('available');
  });

  it('restores with Ctrl+Z and reports the photo', async () => {
    vi.mocked(undoLast).mockResolvedValue({ id: 4, name: 'IMG_4.jpg' });
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(onUndone));
    await waitFor(() => expect(result.current.undoState).toBe('available'));

    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'z', ctrlKey: true }));
    });

    await waitFor(() => expect(onUndone).toHaveBeenCalledWith({ id: 4, name: 'IMG_4.jpg' }));
    expect(result.current.toast).toEqual({ text: 'Przywrócono IMG_4.jpg', kind: 'info' });
  });

  it.each([
    ['occupied', 'Nie można cofnąć: w tym miejscu jest już plik o tej nazwie.'],
    ['missing', 'Nie można cofnąć: plik nie istnieje już w folderze Odrzucone.'],
    ['missingFromTrash', 'Nie można cofnąć: plik nie jest już w Koszu.'],
    [
      'trashUndoUnsupported',
      'Na tym systemie nie można cofnąć przeniesienia do kosza – przywróć zdjęcie z Kosza ręcznie.',
    ],
    ['internal', 'Nie udało się cofnąć odrzucenia.'],
    ['somethingNew', 'Nie udało się cofnąć odrzucenia.'],
  ])('explains a failed undo (%s)', async (code, text) => {
    vi.mocked(undoLast).mockRejectedValue({ code });
    const { result } = renderHook(() => useUndo());

    await act(() => result.current.undo());

    expect(result.current.toast).toEqual({ text, kind: 'error' });
  });

  it('does nothing when there is nothing to undo', async () => {
    vi.mocked(undoLast).mockResolvedValue(null);
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(onUndone));

    await act(() => result.current.undo());

    expect(onUndone).not.toHaveBeenCalled();
    expect(result.current.toast).toBeNull();
  });

  it('only explains when just rejections into an unrestorable trash are left', async () => {
    vi.mocked(undoState).mockResolvedValue('trashOnly');
    const onUndone = vi.fn();
    const { result } = renderHook(() => useUndo(onUndone));
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
    expect(undoLast).not.toHaveBeenCalled();
    expect(onUndone).not.toHaveBeenCalled();
  });

  it('starts with nothing to undo when the state cannot be read', async () => {
    vi.mocked(undoState).mockRejectedValue(new Error('no backend'));
    const { result } = renderHook(() => useUndo());

    await waitFor(() => expect(undoState).toHaveBeenCalled());

    expect(result.current.undoState).toBe('none');
  });
});
