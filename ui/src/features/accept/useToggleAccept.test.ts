import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { acceptPhoto, unacceptPhoto } from './api';
import { useToggleAccept } from './useToggleAccept';

vi.mock('./api', () => ({
  acceptPhoto: vi.fn(),
  unacceptPhoto: vi.fn(),
}));

describe('useToggleAccept', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(acceptPhoto).mockResolvedValue({ name: 'IMG_4.jpg' });
    vi.mocked(unacceptPhoto).mockResolvedValue({ name: 'IMG_4.jpg' });
  });

  it('accepts a photo without the mark', async () => {
    const showToast = vi.fn();
    const { result } = renderHook(() => useToggleAccept(new Set([7]), showToast));

    await act(() => result.current(4));

    expect(acceptPhoto).toHaveBeenCalledWith(4);
    expect(unacceptPhoto).not.toHaveBeenCalled();
    expect(showToast).toHaveBeenCalledWith({ text: 'Zaakceptowano IMG_4.jpg', kind: 'undoable' });
  });

  it('takes the mark off an accepted photo', async () => {
    const showToast = vi.fn();
    const { result } = renderHook(() => useToggleAccept(new Set([4]), showToast));

    await act(() => result.current(4));

    expect(unacceptPhoto).toHaveBeenCalledWith(4);
    expect(showToast).toHaveBeenCalledWith({
      text: 'Cofnięto akceptację IMG_4.jpg',
      kind: 'undoable',
    });
  });

  it('reports a failure', async () => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
    vi.mocked(acceptPhoto).mockRejectedValue({ code: 'internal' });
    const showToast = vi.fn();
    const { result } = renderHook(() => useToggleAccept(new Set(), showToast));

    await act(() => result.current(4));

    expect(showToast).toHaveBeenCalledWith({
      text: 'Nie udało się zmienić akceptacji zdjęcia.',
      kind: 'error',
    });
  });
});
