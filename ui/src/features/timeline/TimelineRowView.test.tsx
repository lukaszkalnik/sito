import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import type { SeriesLabel, TimelineEntry } from './api';
import '../../i18n';
import { TimelineRowView } from './TimelineRowView';

vi.mock('./thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

function entry(id: number, takenAt: string): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt,
    timeSource: 'exif',
    format: 'jpeg',
  };
}

const context = {
  scanId: 1,
  showFolder: false,
  onOpen: vi.fn(),
  acceptedIds: new Set<number>(),
  highlighted: null,
  followed: null,
  revision: 0,
};

function renderHeader(label: SeriesLabel) {
  const entries = [entry(0, '2024-06-01T17:00:22'), entry(1, '2024-06-01T17:00:11')];
  render(
    <TimelineRowView
      row={{ kind: 'series', key: 'series-0', entries, label, edited: false }}
      context={context}
    />,
  );
  return screen.getByRole('heading', { level: 3 });
}

describe('series header', () => {
  it('shows count and time range for a series no other camera overlaps', () => {
    expect(renderHeader({ kind: 'plain' })).toHaveTextContent(/^2 zdjęcia · 17:00:11–17:00:22$/);
  });

  it('names the camera of an overlapping series', () => {
    expect(renderHeader({ kind: 'camera', name: 'Canon EOS R6' })).toHaveTextContent(
      '2 zdjęcia · 17:00:11–17:00:22 · Canon EOS R6',
    );
  });

  it('says the camera is unknown when it is', () => {
    expect(renderHeader({ kind: 'unknownCamera' })).toHaveTextContent(/· nieznany aparat$/);
  });

  it('shows the camera in a block of a short series too', () => {
    const entries = [entry(0, '2024-06-01T17:00:22'), entry(1, '2024-06-01T17:00:11')];
    render(
      <TimelineRowView
        row={{
          kind: 'blocks',
          key: 'blocks-0',
          groups: [
            { kind: 'series', entries, label: { kind: 'camera', name: 'Pixel 8' }, edited: false },
          ],
        }}
        context={context}
      />,
    );

    expect(screen.getByRole('heading', { level: 3 })).toHaveTextContent(/· Pixel 8$/);
  });
});

describe('edited series', () => {
  const entries = [entry(4, '2024-06-01T17:00:22'), entry(5, '2024-06-01T17:00:11')];

  function renderSeries(edited: boolean, onMenu = vi.fn()) {
    render(
      <TimelineRowView
        row={{ kind: 'series', key: 'series-4', entries, label: { kind: 'plain' }, edited }}
        context={{ ...context, onMenu }}
      />,
    );
    return onMenu;
  }

  it('marks an edited series with a pencil', () => {
    renderSeries(true);

    expect(
      screen.getByRole('img', { name: 'Seria edytowana ręcznie – nie zmienia się z odstępem' }),
    ).toBeInTheDocument();
  });

  it('shows no pencil on a series found automatically', () => {
    renderSeries(false);

    expect(screen.queryByRole('img')).not.toBeInTheDocument();
  });

  it('opens the menu of the series from its header, at the pointer', () => {
    const onMenu = renderSeries(false);

    fireEvent.contextMenu(screen.getByRole('heading', { level: 3 }), { clientX: 30, clientY: 40 });

    expect(onMenu).toHaveBeenCalledWith({ target: { kind: 'series', firstId: 4 }, x: 30, y: 40 });
  });
});

describe('tiles', () => {
  it('mark a photo taken out of its series by hand with a pencil', () => {
    const entries = [entry(0, '2024-06-01T17:00:22'), entry(1, '2024-06-01T17:00:11')];
    render(
      <TimelineRowView
        row={{ kind: 'tiles', key: 'tiles-0', entries, inSeries: false, seriesEnd: false }}
        context={{ ...context, pinnedIds: new Set([1]) }}
      />,
    );

    const pencils = screen.getAllByRole('img', {
      name: 'Wyjęte z serii ręcznie – nie łączy się w serię z odstępem',
    });
    expect(pencils).toHaveLength(1);
    expect(screen.getAllByRole('listitem')[1]).toContainElement(pencils[0] ?? null);
  });

  it('open the menu of their photo, below the tile when opened from the keyboard', () => {
    const entries = [entry(0, '2024-06-01T17:00:22')];
    const onMenu = vi.fn();
    render(
      <TimelineRowView
        row={{ kind: 'tiles', key: 'tiles-0', entries, inSeries: false, seriesEnd: false }}
        context={{ ...context, onMenu }}
      />,
    );

    fireEvent.contextMenu(screen.getByRole('button', { name: /IMG_0\.jpg/ }));

    expect(onMenu).toHaveBeenCalledWith({ target: { kind: 'photo', id: 0 }, x: 0, y: 0 });
  });

  it('marks the photo the viewer shows as current, and only that one', () => {
    const entries = [entry(0, '2024-06-01T17:00:22'), entry(1, '2024-06-01T17:00:11')];
    render(
      <TimelineRowView
        row={{ kind: 'tiles', key: 'tiles-0', entries, inSeries: false, seriesEnd: false }}
        context={{ ...context, followed: 1 }}
      />,
    );

    const tiles = screen.getAllByRole('listitem');
    expect(tiles[0]).not.toHaveAttribute('aria-current');
    expect(tiles[1]).toHaveAttribute('aria-current', 'true');
    expect(tiles[1]).toHaveAttribute('title', 'IMG_1.jpg');
  });
});
