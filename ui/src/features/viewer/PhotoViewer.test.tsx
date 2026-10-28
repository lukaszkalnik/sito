import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { Series, TimelineEntry } from '../timeline/api';
import { setWindowTitle } from './api';
import '../../i18n';
import { PhotoViewer } from './PhotoViewer';

vi.mock('./api', () => ({
  photoUrl: (scanId: number, id: number) => `photo://localhost/${id}?scan=${scanId}`,
  setWindowTitle: vi.fn(() => Promise.resolve()),
}));
vi.mock('../timeline/thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

const entries: TimelineEntry[] = Array.from({ length: 6 }, (_, id) => ({
  id,
  name: `IMG_${id}.jpg`,
  relativeDir: id === 0 ? '' : 'Camera',
  relativePath: `IMG_${id}.jpg`,
  takenAt: `2024-06-01T12:00:0${9 - id}`,
  timeSource: 'exif',
  format: 'jpeg',
}));
// Series at positions 1–3; 0, 4 and 5 are single photos.
const series: Series[] = [{ start: 1, len: 3 }];

function renderViewer(position: number, mode: 'series' | 'all' = 'all', canUndo = false) {
  const onShow = vi.fn();
  const onClose = vi.fn();
  const onReject = vi.fn();
  const onUndo = vi.fn();
  render(
    <PhotoViewer
      entries={entries}
      series={series}
      mode={mode}
      scanId={3}
      position={position}
      onShow={onShow}
      onClose={onClose}
      onReject={onReject}
      canUndo={canUndo}
      onUndo={onUndo}
    />,
  );
  return { onShow, onClose, onReject, onUndo };
}

describe('PhotoViewer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('says when the date comes from Google Takeout', () => {
    const datedBy = (timeSource: TimelineEntry['timeSource']) => {
      cleanup();
      render(
        <PhotoViewer
          entries={[
            {
              id: 0,
              name: 'IMG_0.jpg',
              relativeDir: '',
              relativePath: 'IMG_0.jpg',
              takenAt: '2024-06-01T12:00:09',
              timeSource,
              format: 'jpeg',
            },
          ]}
          series={[]}
          mode="all"
          scanId={3}
          position={0}
          onShow={vi.fn()}
          onClose={vi.fn()}
          onReject={vi.fn()}
          canUndo={false}
          onUndo={vi.fn()}
        />,
      );
    };

    datedBy('takeout');
    expect(screen.getByText('data z Google Takeout')).toBeInTheDocument();
    datedBy('takeoutCorrected');
    expect(screen.getByText('data poprawiona w Google Photos')).toBeInTheDocument();
    datedBy('exif');
    expect(screen.queryByText(/Google/)).not.toBeInTheDocument();
  });

  it('shows the photo with its name, time and folder, and titles the window', () => {
    renderViewer(2);

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
    renderViewer(2);
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
      renderViewer(2);
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

  it('hides the spinner when a photo cannot be loaded', () => {
    vi.useFakeTimers();
    try {
      renderViewer(2);
      act(() => vi.advanceTimersByTime(300));

      fireEvent.error(screen.getByTestId('viewer-photo-loading'));

      expect(screen.queryByRole('status', { name: 'Wczytywanie zdjęcia…' })).toBeNull();
    } finally {
      vi.useRealTimers();
    }
  });

  it('reports a photo that cannot be loaded', () => {
    renderViewer(2);

    fireEvent.error(screen.getByTestId('viewer-photo-loading'));

    expect(screen.getByRole('alert')).toHaveTextContent('Nie udało się wczytać zdjęcia.');
  });

  it('hides the previous photo when the next one cannot be loaded', () => {
    const props = {
      entries,
      series,
      mode: 'all' as const,
      scanId: 3,
      onShow: vi.fn(),
      onClose: vi.fn(),
      onReject: vi.fn(),
      canUndo: false,
      onUndo: vi.fn(),
    };
    const { rerender } = render(<PhotoViewer {...props} position={2} />);
    fireEvent.load(screen.getByTestId('viewer-photo-loading'));

    rerender(<PhotoViewer {...props} position={3} />);
    fireEvent.error(screen.getByTestId('viewer-photo-loading'));

    expect(screen.getByRole('alert')).toHaveTextContent('Nie udało się wczytać zdjęcia.');
    expect(screen.queryByRole('img', { name: /IMG_\d\.jpg/ })).toBeNull();
  });

  it('steps to the previous and next photo with buttons and arrow keys', async () => {
    const { onShow } = renderViewer(2);

    await userEvent.click(screen.getByRole('button', { name: 'Następne zdjęcie' }));
    expect(onShow).toHaveBeenLastCalledWith(3);
    await userEvent.click(screen.getByRole('button', { name: 'Poprzednie zdjęcie' }));
    expect(onShow).toHaveBeenLastCalledWith(1);

    await userEvent.keyboard('{ArrowRight}');
    expect(onShow).toHaveBeenLastCalledWith(3);
    await userEvent.keyboard('{ArrowLeft}');
    expect(onShow).toHaveBeenLastCalledWith(1);
  });

  it('leaves the series in the all view', async () => {
    const all = renderViewer(3, 'all');
    await userEvent.click(screen.getByRole('button', { name: 'Następne zdjęcie' }));
    expect(all.onShow).toHaveBeenLastCalledWith(4);
  });

  it('stops at the ends of the series view', () => {
    renderViewer(3, 'series');

    expect(screen.getByRole('button', { name: 'Następne zdjęcie' })).toBeDisabled();
    expect(screen.getByRole('button', { name: 'Poprzednie zdjęcie' })).toBeEnabled();
  });

  it('shows the series strip and opens a photo from it', async () => {
    const { onShow } = renderViewer(2);
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

  it('has no strip for a single photo, and names the root folder', () => {
    renderViewer(0);

    expect(screen.queryByRole('navigation')).toBeNull();
    expect(screen.getByText('folder główny')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Poprzednie zdjęcie' })).toBeDisabled();
  });

  it('rejects the photo with the button and the Delete key', async () => {
    const { onReject } = renderViewer(2);

    await userEvent.click(screen.getByRole('button', { name: 'Odrzuć' }));
    await userEvent.keyboard('{Delete}');

    expect(onReject).toHaveBeenCalledTimes(2);
  });

  it('offers Cofnij only when there is something to undo', async () => {
    renderViewer(2);
    expect(screen.getByRole('button', { name: 'Cofnij' })).toBeDisabled();
    cleanup();

    const { onUndo } = renderViewer(2, 'all', true);
    await userEvent.click(screen.getByRole('button', { name: 'Cofnij' }));

    expect(onUndo).toHaveBeenCalledOnce();
  });

  it('closes with Escape and the close button', async () => {
    const { onClose } = renderViewer(2);

    await userEvent.keyboard('{Escape}');
    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));

    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
