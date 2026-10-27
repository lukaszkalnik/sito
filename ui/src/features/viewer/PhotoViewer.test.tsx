import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import type { Series, TimelineEntry } from '../../api/scan';
import { setWindowTitle } from '../../api/viewer';
import '../../i18n';
import { PhotoViewer } from './PhotoViewer';

vi.mock('../../api/viewer', () => ({
  photoUrl: (scanId: number, id: number) => `photo://localhost/${id}?scan=${scanId}`,
  setWindowTitle: vi.fn(() => Promise.resolve()),
}));
vi.mock('../../api/thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

const entries: TimelineEntry[] = Array.from({ length: 6 }, (_, id) => ({
  id,
  name: `IMG_${id}.jpg`,
  relativeDir: id === 0 ? '' : 'Camera',
  relativePath: `IMG_${id}.jpg`,
  takenAt: `2024-06-01T12:00:0${9 - id}`,
  fromFileDate: false,
}));
// Series at positions 1–3; 0, 4 and 5 are single photos.
const series: Series[] = [{ start: 1, len: 3 }];

function renderViewer(position: number, mode: 'series' | 'all' = 'all') {
  const onShow = vi.fn();
  const onClose = vi.fn();
  render(
    <PhotoViewer
      entries={entries}
      series={series}
      mode={mode}
      scanId={3}
      position={position}
      onShow={onShow}
      onClose={onClose}
    />,
  );
  return { onShow, onClose };
}

describe('PhotoViewer', () => {
  beforeEach(() => {
    vi.clearAllMocks();
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

  it('reports a photo that cannot be loaded', () => {
    renderViewer(2);

    fireEvent.error(screen.getByTestId('viewer-photo-loading'));

    expect(screen.getByRole('alert')).toHaveTextContent('Nie udało się wczytać zdjęcia.');
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

  it('closes with Escape and the close button', async () => {
    const { onClose } = renderViewer(2);

    await userEvent.keyboard('{Escape}');
    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));

    expect(onClose).toHaveBeenCalledTimes(2);
  });
});
