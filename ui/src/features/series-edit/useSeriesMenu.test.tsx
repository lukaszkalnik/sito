import { act, renderHook } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import '../../i18n';
import type { TimelineEntry } from '../timeline/api';
import { editSeries } from './api';
import type { Arranged } from './editOptions';
import { useSeriesMenu } from './useSeriesMenu';

vi.mock('./api', async (original) => ({
  ...(await original<typeof import('./api')>()),
  editSeries: vi.fn(),
}));

function entry(id: number): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt: '2024-06-01T12:00:00',
    timeSource: 'exif',
    format: 'jpeg',
  };
}

/** Photos 0–1 a series, 2 a single photo. */
const arranged: Arranged = {
  entries: [entry(0), entry(1), entry(2)],
  series: [{ start: 0, len: 2, label: { kind: 'plain' }, edited: false }],
  pinnedIds: new Set(),
};

function renderMenu(showToast = vi.fn()) {
  return renderHook(() => useSeriesMenu(arranged, 12, showToast));
}

describe('useSeriesMenu', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    vi.mocked(editSeries).mockResolvedValue({ name: 'IMG_1.jpg' });
  });

  it('opens with the edits of the photo, where it was asked for', () => {
    const { result } = renderMenu();

    act(() => result.current.open({ target: { kind: 'photo', id: 1 }, x: 5, y: 6 }));

    expect(result.current.menu?.x).toBe(5);
    expect(result.current.menu?.options.map((o) => o.action)).toEqual(['cut', 'remove', 'ungroup']);
  });

  it('stays closed when there is nothing to edit', () => {
    const { result } = renderHook(() => useSeriesMenu({ ...arranged, series: [] }, 10, vi.fn()));

    act(() => result.current.open({ target: { kind: 'photo', id: 2 }, x: 0, y: 0 }));

    expect(result.current.menu).toBeNull();
  });

  it('makes the chosen edit for the gap shown and confirms it', async () => {
    const showToast = vi.fn();
    const { result } = renderMenu(showToast);
    act(() => result.current.open({ target: { kind: 'photo', id: 1 }, x: 0, y: 0 }));
    const remove = result.current.menu?.options[1];

    await act(() => result.current.choose(remove!));

    expect(editSeries).toHaveBeenCalledWith({ kind: 'remove', photo: 1 }, 12);
    expect(result.current.menu).toBeNull();
    expect(showToast).toHaveBeenCalledWith({ text: 'Wyjęto IMG_1.jpg z serii', kind: 'info' });
  });

  it('explains an edit refused because the series changed', async () => {
    vi.mocked(editSeries).mockRejectedValue({ code: 'seriesChanged' });
    const showToast = vi.fn();
    const { result } = renderMenu(showToast);

    await act(() =>
      result.current.choose({ action: 'ungroup', edit: { kind: 'ungroup', photo: 0 } }),
    );

    expect(showToast).toHaveBeenCalledWith({
      text: 'Nie można zmienić serii – zmieniła się w międzyczasie.',
      kind: 'error',
    });
  });

  it('gives the focus back to where it was when closed from the keyboard', () => {
    const tile = document.createElement('button');
    document.body.append(tile);
    tile.focus();
    const { result } = renderMenu();
    act(() => result.current.open({ target: { kind: 'photo', id: 1 }, x: 0, y: 0 }));
    tile.blur();

    act(() => result.current.close(true));

    expect(tile).toHaveFocus();
    tile.remove();
  });
});
