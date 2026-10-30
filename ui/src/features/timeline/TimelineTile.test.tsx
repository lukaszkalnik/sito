import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import type { TimelineEntry } from './api';
import '../../i18n';
import { forgetLoaded, rememberLoaded } from './loadedThumbnails';
import { THUMBNAIL_DELAY_MS, THUMBNAIL_SLOTS } from './thumbnailQueue';
import { TimelineTile } from './TimelineTile';

vi.mock('./thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

const entry: TimelineEntry = {
  id: 7,
  name: 'IMG_7.jpg',
  relativeDir: 'Camera',
  relativePath: 'Camera\\IMG_7.jpg',
  takenAt: '2024-06-01T12:00:05',
  timeSource: 'file',
  format: 'jpeg',
};

function renderTile(scanId = 3) {
  const { container, rerender } = render(
    <ul>
      <TimelineTile entry={entry} scanId={scanId} showFolder onOpen={vi.fn()} />
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

  it('marks an accepted photo with a check, and only that one', () => {
    renderTile();
    expect(screen.queryByRole('img', { name: 'Zaakceptowane' })).not.toBeInTheDocument();
    cleanup();

    render(
      <ul>
        <TimelineTile entry={entry} scanId={3} showFolder onOpen={vi.fn()} accepted />
      </ul>,
    );

    expect(screen.getByRole('img', { name: 'Zaakceptowane' })).toBeInTheDocument();
  });

  it.each([
    ['Pulpit\\2024', 'Pulpit\\2024'],
    ['', 'folder główny'],
  ])('shows the folder %j to tell same-named photos apart', (relativeDir, text) => {
    render(
      <ul>
        <TimelineTile entry={{ ...entry, relativeDir }} scanId={3} showFolder onOpen={vi.fn()} />
      </ul>,
    );

    expect(screen.getByTestId('timeline-tile-folder')).toHaveTextContent(text);
  });

  it('opens the photo when clicked', () => {
    const onOpen = vi.fn();
    render(
      <ul>
        <TimelineTile entry={entry} scanId={3} showFolder={false} onOpen={onOpen} />
      </ul>,
    );

    fireEvent.click(screen.getByRole('button', { name: 'Otwórz IMG_7.jpg' }));

    expect(onOpen).toHaveBeenCalledWith(entry);
  });

  it('hides the folder when all photos share it', () => {
    render(
      <ul>
        <TimelineTile entry={entry} scanId={3} showFolder={false} onOpen={vi.fn()} />
      </ul>,
    );

    expect(screen.queryByTestId('timeline-tile-folder')).toBeNull();
    expect(screen.getByText('IMG_7.jpg')).toBeInTheDocument();
  });

  it('requests the thumbnail only after the tile stayed on screen for a moment', () => {
    const { image } = renderTile();
    expect(image()).toBeNull();

    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));

    expect(image()).toHaveAttribute('src', 'thumb://localhost/7?scan=3');
    expect(screen.getByTestId('timeline-tile')).toHaveClass('timeline-tile--loading');
  });

  it('lets the next tile load once a thumbnail has arrived', () => {
    const { container } = render(
      <ul>
        {[1, 2, 3, 4].map((id) => (
          <TimelineTile
            key={id}
            entry={{ ...entry, id }}
            scanId={3}
            showFolder={false}
            onOpen={vi.fn()}
          />
        ))}
      </ul>,
    );
    const images = () => [...container.querySelectorAll('img')];
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));
    expect(images()).toHaveLength(THUMBNAIL_SLOTS);

    const [first] = images();
    if (first) fireEvent.load(first);

    expect(images()).toHaveLength(THUMBNAIL_SLOTS + 1);
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
        <TimelineTile entry={entry} scanId={4} showFolder onOpen={vi.fn()} />
      </ul>,
    );
    // The new scan's thumbnail takes its turn in the queue like any other.
    expect(image()).toBeNull();
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));

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

  it('shows a portrait thumbnail uncropped once it has loaded', () => {
    const { image } = renderTile();
    act(() => vi.advanceTimersByTime(THUMBNAIL_DELAY_MS));
    expect(screen.getByTestId('timeline-tile')).toHaveAttribute('data-shape', 'landscape');

    const img = image()!;
    Object.defineProperty(img, 'naturalWidth', { value: 1360 });
    Object.defineProperty(img, 'naturalHeight', { value: 2008 });
    fireEvent.load(img);

    expect(screen.getByTestId('timeline-tile')).toHaveAttribute('data-shape', 'portrait');
    expect(screen.getByTestId('timeline-tile')).toHaveClass('timeline-tile--portrait');
  });

  it('shows a portrait thumbnail seen before as portrait at once', () => {
    rememberLoaded('thumb://localhost/7?scan=3', 'portrait');

    renderTile();

    expect(screen.getByTestId('timeline-tile')).toHaveAttribute('data-shape', 'portrait');
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

  it('names RAW photos and RAW+JPEG pairs', () => {
    const tile = (format: TimelineEntry['format']) => {
      cleanup();
      render(
        <ul>
          <TimelineTile
            entry={{ ...entry, timeSource: 'exif', format }}
            scanId={3}
            showFolder
            onOpen={vi.fn()}
          />
        </ul>,
      );
    };

    tile('rawJpeg');
    expect(screen.getByText('RAW+JPEG')).toBeInTheDocument();
    tile('raw');
    expect(screen.getByText('RAW')).toBeInTheDocument();
    tile('jpeg');
    expect(screen.queryByText(/RAW/)).not.toBeInTheDocument();
  });
});
