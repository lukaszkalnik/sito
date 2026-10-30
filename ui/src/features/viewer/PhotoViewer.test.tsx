import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TimelineEntry } from '../timeline/api';
import { setWindowTitle, type ViewerPhoto } from './api';
import '../../i18n';
import { PhotoViewer } from './PhotoViewer';

vi.mock('./api', () => ({
  photoUrl: (scanId: number, id: number) => `photo://localhost/${id}?scan=${scanId}`,
  setWindowTitle: vi.fn(() => Promise.resolve()),
}));
vi.mock('../timeline/thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

/** Photo `id` of six, one second apart, newest first. */
function entry(id: number, timeSource: TimelineEntry['timeSource'] = 'exif'): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: id === 0 ? '' : 'Camera',
    relativePath: `IMG_${id}.jpg`,
    takenAt: `2024-06-01T12:00:0${9 - id}`,
    timeSource,
    format: 'jpeg',
  };
}

/**
 * The page of photo `id` in the all view of six photos, where photos 1–3
 * form a series and 0, 4 and 5 are single photos.
 */
function pageOf(id: number, changes: Partial<ViewerPhoto> = {}): ViewerPhoto {
  const inSeries = id >= 1 && id <= 3;
  return {
    entry: entry(id),
    previous: id > 0 ? id - 1 : null,
    next: id < 5 ? id + 1 : null,
    series: inSeries ? [1, 2, 3].map((member) => entry(member)) : [],
    afterReject: null,
    ...changes,
  };
}

function viewerProps(photo: ViewerPhoto, canUndo = false) {
  return {
    photo,
    scanId: 3,
    onShow: vi.fn(),
    onClose: vi.fn(),
    onReject: vi.fn(),
    rejectTo: 'folder' as const,
    canUndo,
    onUndo: vi.fn(),
  };
}

function renderViewer(photo: ViewerPhoto, canUndo = false) {
  const props = viewerProps(photo, canUndo);
  render(<PhotoViewer {...props} />);
  return props;
}

describe('PhotoViewer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('says when the date comes from Google Takeout', () => {
    const datedBy = (timeSource: TimelineEntry['timeSource']) => {
      cleanup();
      renderViewer(pageOf(0, { entry: entry(0, timeSource) }));
    };
    datedBy('takeout');
    expect(screen.getByText('data z Google Takeout')).toBeInTheDocument();
    datedBy('takeoutCorrected');
    expect(screen.getByText('data poprawiona w Google Photos')).toBeInTheDocument();
    datedBy('exif');
    expect(screen.queryByText(/Google/)).not.toBeInTheDocument();
  });

  it('shows the photo with its name, time and folder, and titles the window', () => {
    renderViewer(pageOf(2));

    expect(screen.getByTestId('viewer-photo-loading')).toHaveAttribute(
      'src',
      'photo://localhost/2?scan=3',
    );
    expect(screen.getByText('IMG_2.jpg')).toBeInTheDocument();
    expect(screen.getByText(/12:00:07/)).toBeInTheDocument();
    expect(screen.getByText('Camera')).toBeInTheDocument();
    expect(setWindowTitle).toHaveBeenCalledWith('IMG_2.jpg – Sito');
  });

  it('keeps the shown photo until the next one has loaded', () => {
    renderViewer(pageOf(2));
    const loading = screen.getByTestId('viewer-photo-loading');

    fireEvent.load(loading);

    expect(screen.getByRole('img', { name: 'IMG_2.jpg' })).toHaveAttribute(
      'src',
      'photo://localhost/2?scan=3',
    );
    expect(screen.queryByTestId('viewer-photo-loading')).toBeNull();
  });

  it('shows a spinner while a photo takes long to load', () => {
    vi.useFakeTimers();
    try {
      renderViewer(pageOf(2));
      const spinner = () => screen.queryByRole('status', { name: 'Wczytywanie zdjęcia…' });
      expect(spinner()).toBeNull();

      act(() => vi.advanceTimersByTime(300));
      expect(spinner()).toBeInTheDocument();

      fireEvent.load(screen.getByTestId('viewer-photo-loading'));
      expect(spinner()).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it('replaces the previous photo with the spinner when the next one is slow', () => {
    vi.useFakeTimers();
    try {
      const { rerender } = render(<PhotoViewer {...viewerProps(pageOf(2))} />);
      fireEvent.load(screen.getByTestId('viewer-photo-loading'));
      rerender(<PhotoViewer {...viewerProps(pageOf(3))} />);

      // At first the previous photo stays, so fast photos never flash.
      expect(screen.getByRole('img', { name: 'IMG_3.jpg' })).toHaveAttribute(
        'src',
        'photo://localhost/2?scan=3',
      );

      act(() => vi.advanceTimersByTime(300));

      expect(screen.getByRole('status', { name: 'Wczytywanie zdjęcia…' })).toBeInTheDocument();
      expect(screen.queryByRole('img', { name: 'IMG_3.jpg' })).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it('hides the spinner when a photo cannot be loaded', () => {
    vi.useFakeTimers();
    try {
      renderViewer(pageOf(2));
      act(() => vi.advanceTimersByTime(300));

      fireEvent.error(screen.getByTestId('viewer-photo-loading'));

      expect(screen.queryByRole('status', { name: 'Wczytywanie zdjęcia…' })).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it('reports a photo that cannot be loaded', () => {
    renderViewer(pageOf(2));

    fireEvent.error(screen.getByTestId('viewer-photo-loading'));

    expect(screen.getByRole('alert')).toHaveTextContent('Nie udało się wczytać zdjęcia.');
  });

  it('hides the previous photo when the next one cannot be loaded', () => {
    const { rerender } = render(<PhotoViewer {...viewerProps(pageOf(2))} />);
    fireEvent.load(screen.getByTestId('viewer-photo-loading'));

    rerender(<PhotoViewer {...viewerProps(pageOf(3))} />);
    fireEvent.error(screen.getByTestId('viewer-photo-loading'));

    expect(screen.getByRole('alert')).toHaveTextContent('Nie udało się wczytać zdjęcia.');
    expect(screen.queryByRole('img', { name: /IMG_\d\.jpg/ })).toBeNull();
  });

  it('steps to the previous and next photo with buttons and arrow keys', async () => {
    const { onShow } = renderViewer(pageOf(2));

    await userEvent.click(screen.getByRole('button', { name: 'Następne zdjęcie' }));
    expect(onShow).toHaveBeenLastCalledWith(3);
    await userEvent.click(screen.getByRole('button', { name: 'Poprzednie zdjęcie' }));
    expect(onShow).toHaveBeenLastCalledWith(1);

    await userEvent.keyboard('{ArrowRight}');
    expect(onShow).toHaveBeenLastCalledWith(3);
    await userEvent.keyboard('{ArrowLeft}');
    expect(onShow).toHaveBeenLastCalledWith(1);
  });

  it('stops at the ends of the view', () => {
    renderViewer(pageOf(3, { next: null }));

    expect(screen.getByRole('button', { name: 'Następne zdjęcie' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Poprzednie zdjęcie' })).toBeEnabled();
  });

  it('shows the series strip and opens a photo from it', async () => {
    const { onShow } = renderViewer(pageOf(2));
    const strip = screen.getByRole('navigation', { name: 'Zdjęcia w serii' });
    const items = strip.querySelectorAll('button');

    expect(items).toHaveLength(3);
    expect(screen.getByRole('button', { name: 'IMG_2.jpg' })).toHaveAttribute(
      'aria-current',
      'true',
    );

    await userEvent.click(screen.getByRole('button', { name: 'IMG_3.jpg' }));
    expect(onShow).toHaveBeenLastCalledWith(3);
  });

  it('scrolls the strip to keep the current photo visible', () => {
    const scrolled = vi.spyOn(Element.prototype, 'scrollIntoView');
    const props = viewerProps(pageOf(1));
    const { rerender } = render(<PhotoViewer {...props} />);
    scrolled.mockClear();

    rerender(<PhotoViewer {...props} photo={pageOf(3)} />);

    expect(scrolled).toHaveBeenCalledTimes(1);
    expect(scrolled.mock.contexts[0]).toBe(screen.getByRole('button', { name: 'IMG_3.jpg' }));
    expect(scrolled).toHaveBeenCalledWith({ inline: 'nearest', block: 'nearest' });
    scrolled.mockRestore();
  });

  it('has no strip for a single photo, and names the root folder', () => {
    renderViewer(pageOf(0));

    expect(screen.queryByRole('navigation')).toBeNull();
    expect(screen.getByText('folder główny')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Poprzednie zdjęcie' })).toBeDisabled();
  });

  it('rejects the photo with the button and the Delete key', async () => {
    const { onReject } = renderViewer(pageOf(2));

    await userEvent.click(screen.getByRole('button', { name: 'Odrzuć' }));
    await userEvent.keyboard('{Delete}');

    expect(onReject).toHaveBeenCalledTimes(2);
  });

  it('offers Cofnij only when there is something to undo', async () => {
    renderViewer(pageOf(2));
    expect(screen.getByRole('button', { name: 'Cofnij' })).toBeDisabled();
    cleanup();

    const { onUndo } = renderViewer(pageOf(2), true);
    await userEvent.click(screen.getByRole('button', { name: 'Cofnij' }));

    expect(onUndo).toHaveBeenCalledOnce();
  });

  it('closes with Escape and the close button', async () => {
    const { onClose } = renderViewer(pageOf(2));

    await userEvent.keyboard('{Escape}');
    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));

    expect(onClose).toHaveBeenCalledTimes(2);
  });
});

/** A view of 1000×500 pixels, as jsdom does no layout. */
class FixedSizeObserver {
  private readonly callback: ResizeObserverCallback;
  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
  }
  observe() {
    const entry = { contentRect: { width: 1000, height: 500 } } as ResizeObserverEntry;
    this.callback([entry], this as unknown as ResizeObserver);
  }
  disconnect() {}
  unobserve() {}
}

/** Lets the loading photo finish as a 2000×1000 image. */
function loadPhoto() {
  const image = screen.getByTestId('viewer-photo-loading');
  Object.defineProperty(image, 'naturalWidth', { value: 2000 });
  Object.defineProperty(image, 'naturalHeight', { value: 1000 });
  fireEvent.load(image);
}

const zoomBadge = () => screen.queryByRole('group', { name: 'Powiększenie' });

describe('PhotoViewer zoom', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', FixedSizeObserver);
    vi.clearAllMocks();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('shows the zoom level only while zoomed', async () => {
    renderViewer(pageOf(2));
    loadPhoto();
    expect(zoomBadge()).toBeNull();

    await userEvent.keyboard('1');
    expect(zoomBadge()).toHaveTextContent('100%');
    await userEvent.keyboard('+');
    expect(zoomBadge()).toHaveTextContent('125%');

    await userEvent.keyboard('0');
    expect(zoomBadge()).toBeNull();
  });

  it('goes back to fit with „Dopasuj”', async () => {
    renderViewer(pageOf(2));
    loadPhoto();
    await userEvent.keyboard('1');

    await userEvent.click(screen.getByRole('button', { name: 'Dopasuj' }));

    expect(zoomBadge()).toBeNull();
  });

  it('pans with Shift+arrows instead of stepping', async () => {
    const { onShow } = renderViewer(pageOf(2));
    loadPhoto();
    await userEvent.keyboard('1');

    await userEvent.keyboard('{Shift>}{ArrowRight}{/Shift}');

    expect(onShow).not.toHaveBeenCalled();
    expect(screen.getByRole('img', { name: 'IMG_2.jpg' }).style.transform).toBe(
      'translate(-620px, -250px)',
    );
  });

  it('keeps the zoom within a series and resets it outside', async () => {
    const props = viewerProps(pageOf(2));
    const { rerender } = render(<PhotoViewer {...props} />);
    loadPhoto();
    await userEvent.keyboard('1');

    rerender(<PhotoViewer {...props} photo={pageOf(3)} />);
    expect(zoomBadge()).toHaveTextContent('100%');

    rerender(<PhotoViewer {...props} photo={pageOf(4)} />);
    expect(zoomBadge()).toBeNull();
  });
});
