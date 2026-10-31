import type { Virtualizer } from '@tanstack/react-virtual';
import { act, renderHook } from '@testing-library/react';
import type { UIEvent } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TimelineEntry } from './api';
import { buildRows, DAY_ROW_HEIGHT, TILE_GAP, TILE_ROW_HEIGHT } from './layout';
import type { Reveal } from './reveal';
import { FOLLOW_DELAY_MS, useGridScroll } from './useGridScroll';
import { SAVE_DELAY_MS, type SavedPosition } from './useSavedPosition';

const VIEW_HEIGHT = 600;
const TILE_HEIGHT = TILE_ROW_HEIGHT - TILE_GAP;

function entry(id: number): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt: `2024-06-01T12:${String(59 - id).padStart(2, '0')}:00`,
    timeSource: 'exif',
    format: 'jpeg',
  };
}

// One column: a day header, then one row per photo.
const rows = buildRows(
  Array.from({ length: 40 }, (_, id) => entry(id)),
  [],
  1,
  'all',
);
const topOf = (id: number) => DAY_ROW_HEIGHT + id * TILE_ROW_HEIGHT;
const centred = (id: number) => topOf(id) - (VIEW_HEIGHT - TILE_HEIGHT) / 2;
const virtualizer = { measure: vi.fn() } as unknown as Virtualizer<HTMLDivElement, Element>;

interface Props {
  follow: number | null;
  reveal: Reveal | null;
  measured?: boolean;
}

function renderGridScroll(element: HTMLElement, initial: Props, saved?: SavedPosition) {
  const scrollRef = { current: element };
  return renderHook(
    ({ follow, reveal, measured = true }: Props) =>
      useGridScroll(scrollRef, rows, virtualizer, {
        reveal,
        onRevealed: undefined,
        follow,
        saved,
        measured,
      }),
    { initialProps: initial },
  );
}

/** A scroll container with a real `scrollTop`, which jsdom does not keep. */
function scrollElement(): HTMLElement {
  let scrollTop = 0;
  return {
    clientHeight: VIEW_HEIGHT,
    get scrollTop() {
      return scrollTop;
    },
    set scrollTop(value: number) {
      scrollTop = value;
    },
  } as unknown as HTMLElement;
}

describe('useGridScroll following the viewer', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it('leaves the view alone when the followed photo is visible', () => {
    const element = scrollElement();
    renderGridScroll(element, { follow: 1, reveal: null });

    act(() => vi.advanceTimersByTime(FOLLOW_DELAY_MS));

    expect(element.scrollTop).toBe(0);
  });

  it('scrolls to a hidden photo only after a short pause, without highlighting it', () => {
    const element = scrollElement();
    const { result } = renderGridScroll(element, { follow: 30, reveal: null });

    act(() => vi.advanceTimersByTime(FOLLOW_DELAY_MS - 1));
    expect(element.scrollTop).toBe(0);
    act(() => vi.advanceTimersByTime(1));

    expect(element.scrollTop).toBe(centred(30));
    expect(result.current.highlighted).toBeNull();
  });

  it('scrolls only to the last of fast steps', () => {
    const element = scrollElement();
    const { rerender } = renderGridScroll(element, { follow: 20, reveal: null });
    const spy = vi.spyOn(element, 'scrollTop', 'set');

    for (const id of [21, 22, 23]) {
      act(() => vi.advanceTimersByTime(FOLLOW_DELAY_MS / 2));
      rerender({ follow: id, reveal: null });
    }
    act(() => vi.advanceTimersByTime(FOLLOW_DELAY_MS));

    expect(spy).toHaveBeenCalledTimes(1);
    expect(spy).toHaveBeenCalledWith(centred(23));
  });

  it('does nothing for a photo the grid does not show', () => {
    const element = scrollElement();
    renderGridScroll(element, { follow: 99, reveal: null });

    act(() => vi.advanceTimersByTime(FOLLOW_DELAY_MS));

    expect(element.scrollTop).toBe(0);
  });

  it('reveals the photo last shown in the viewer and highlights it for long', () => {
    const element = scrollElement();
    const reveal: Reveal = { id: 30, reason: 'viewerClosed' };
    const { result } = renderGridScroll(element, { follow: null, reveal });

    expect(element.scrollTop).toBe(centred(30));
    expect(result.current.highlighted).toEqual(reveal);
    act(() => vi.advanceTimersByTime(3000));
    expect(result.current.highlighted).toEqual(reveal);
    act(() => vi.advanceTimersByTime(3000));
    expect(result.current.highlighted).toBeNull();
  });
});

describe('useGridScroll keeping the position of the folder', () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  const stored = { path: 'IMG_20.jpg', takenAt: '2024-06-01T12:39:00', offset: 10 };

  function storage(position: typeof stored | null) {
    return { load: vi.fn(() => position), save: vi.fn() };
  }

  /** Scrolls like the user: the element moves, then its scroll event reaches the handler. */
  function userScroll(
    element: HTMLElement,
    onScroll: (event: UIEvent<HTMLElement>) => void,
    top: number,
  ) {
    element.scrollTop = top;
    act(() => onScroll({ currentTarget: element } as unknown as UIEvent<HTMLElement>));
  }

  it('restores the stored photo once the width is measured', () => {
    const element = scrollElement();
    const saved = storage(stored);
    const { rerender } = renderGridScroll(
      element,
      { follow: null, reveal: null, measured: false },
      saved,
    );
    expect(element.scrollTop).toBe(0);

    rerender({ follow: null, reveal: null, measured: true });

    expect(element.scrollTop).toBe(topOf(20) + 10);
    expect(saved.load).toHaveBeenCalledTimes(1);
  });

  it('keeps where the user scrolled before the restore', () => {
    const element = scrollElement();
    const saved = storage(stored);
    const { result, rerender } = renderGridScroll(
      element,
      { follow: null, reveal: null, measured: false },
      saved,
    );

    userScroll(element, result.current.onScroll, 300);
    rerender({ follow: null, reveal: null, measured: true });

    expect(element.scrollTop).toBe(300);
  });

  it('saves the position once scrolling settles', () => {
    const element = scrollElement();
    const saved = storage(null);
    const { result } = renderGridScroll(element, { follow: null, reveal: null }, saved);

    userScroll(element, result.current.onScroll, topOf(5) + 30);
    act(() => vi.advanceTimersByTime(SAVE_DELAY_MS - 1));
    expect(saved.save).not.toHaveBeenCalled();
    act(() => vi.advanceTimersByTime(1));

    expect(saved.save).toHaveBeenCalledWith({
      path: 'IMG_5.jpg',
      takenAt: '2024-06-01T12:54:00',
      offset: 30,
    });
  });

  it('saves the position right away when the grid goes away', () => {
    const element = scrollElement();
    const saved = storage(null);
    const { result, unmount } = renderGridScroll(element, { follow: null, reveal: null }, saved);

    userScroll(element, result.current.onScroll, topOf(5));
    unmount();

    expect(saved.save).toHaveBeenCalledTimes(1);
    expect(saved.save).toHaveBeenCalledWith(
      expect.objectContaining({ path: 'IMG_5.jpg', offset: 0 }),
    );
  });

  it('never saves before the stored position was restored', () => {
    const element = scrollElement();
    const saved = storage(stored);
    const { unmount } = renderGridScroll(
      element,
      { follow: null, reveal: null, measured: false },
      saved,
    );

    unmount();

    expect(saved.save).not.toHaveBeenCalled();
  });
});
