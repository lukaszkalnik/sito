import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TimelineEntry } from '../../api/scan';
import '../../i18n';
import { forgetLoaded } from './loadedThumbnails';
import { THUMBNAIL_DELAY_MS, TimelineTile } from './TimelineTile';

vi.mock('../../api/thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

const entry: TimelineEntry = {
  id: 7,
  name: 'IMG_7.jpg',
  relativeDir: 'Camera',
  relativePath: 'Camera\\IMG_7.jpg',
  takenAt: '2024-06-01T12:00:05',
  fromFileDate: true,
};

function renderTile(scanId = 3) {
  const { container, rerender } = render(
    <ul>
      <TimelineTile entry={entry} scanId={scanId} />
    </ul>,
  );
  const image = () => container.querySelector('img');
  return { image, rerender };
}

describe('TimelineTile', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    forgetLoaded();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows time, name and file-date badge right away', () => {
    renderTile();

    expect(screen.getByText('12:00:05')).toBeInTheDocument();
    expect(screen.getByText('IMG_7.jpg')).toBeInTheDocument();
    expect(screen.getByText('data pliku')).toBeInTheDocument();
    expect(screen.getByTestId('timeline-tile')).toHaveAttribute('title', 'Camera\\IMG_7.jpg');
  });

  it('requests the thumbnail only after the tile stayed on screen for a moment', () => {
    const { image } = renderTile();
    expect(image()).toBeNull();

    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));

    expect(image()).toHaveAttribute('src', 'thumb://localhost/7?scan=3');
    expect(screen.getByTestId('timeline-tile')).toHaveClass('timeline-tile--loading');
  });

  it('marks the tile as loaded when the thumbnail arrives', () => {
    const { image } = renderTile();
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));

    fireEvent.load(image()!);

    expect(screen.getByTestId('timeline-tile')).toHaveClass('timeline-tile--loaded');
  });

  it('shows a hint instead of a broken image when loading fails', () => {
    const { image } = renderTile();
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));

    fireEvent.error(image()!);

    expect(image()).toBeNull();
    expect(
      screen.getByRole('img', { name: 'Nie udało się wczytać miniatury.' }),
    ).toBeInTheDocument();
    expect(screen.getByText('IMG_7.jpg')).toBeInTheDocument();
  });

  it('retries when the tile is reused for a new scan', () => {
    const { image, rerender } = renderTile(3);
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));
    fireEvent.error(image()!);

    rerender(
      <ul>
        <TimelineTile entry={entry} scanId={4} />
      </ul>,
    );

    expect(image()).toHaveAttribute('src', 'thumb://localhost/7?scan=4');
  });

  it('shows a thumbnail seen before at once when its tile comes back', () => {
    const first = renderTile();
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));
    fireEvent.load(first.image()!);
    cleanup();

    const again = renderTile();

    expect(again.image()).toHaveAttribute('src', 'thumb://localhost/7?scan=3');
    expect(screen.getByTestId('timeline-tile')).toHaveClass(
      'timeline-tile--loaded',
      'timeline-tile--instant',
    );
  });

  it('still waits for thumbnails of another scan', () => {
    const first = renderTile(3);
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));
    fireEvent.load(first.image()!);
    cleanup();

    const again = renderTile(4);

    expect(again.image()).toBeNull();
    expect(screen.getByTestId('timeline-tile')).toHaveClass('timeline-tile--loading');
  });
});
