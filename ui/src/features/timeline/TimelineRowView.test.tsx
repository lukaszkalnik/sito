import { render, screen } from '@testing-library/react';
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
};

function renderHeader(label: SeriesLabel) {
  const entries = [entry(0, '2024-06-01T17:00:22'), entry(1, '2024-06-01T17:00:11')];
  render(
    <TimelineRowView row={{ kind: 'series', key: 'series-0', entries, label }} context={context} />,
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
          groups: [{ kind: 'series', entries, label: { kind: 'camera', name: 'Pixel 8' } }],
        }}
        context={context}
      />,
    );

    expect(screen.getByRole('heading', { level: 3 })).toHaveTextContent(/· Pixel 8$/);
  });
});
