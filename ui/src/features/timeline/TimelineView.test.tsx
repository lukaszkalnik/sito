import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getSeries, getTimeline, type Series, type TimelineEntry } from '../../api/scan';
import '../../i18n';
import { TimelineView } from './TimelineView';

vi.mock('../../api/scan', async (importActual) => ({
  ...(await importActual<typeof import('../../api/scan')>()),
  getTimeline: vi.fn(),
  getSeries: vi.fn(),
}));

const mockedTimeline = vi.mocked(getTimeline);
const mockedSeries = vi.mocked(getSeries);

function entries(count: number): TimelineEntry[] {
  return Array.from({ length: count }, (_, id) => ({
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt: '2024-06-01T12:00:00',
    fromFileDate: false,
  }));
}

/** `count` series of two photos each. */
function pairs(count: number): Series[] {
  return Array.from({ length: count }, (_, i) => ({ start: 2 * i, len: 2 }));
}

function renderView(photos: number) {
  render(
    <TimelineView
      path="C:\Zdjęcia"
      summary={{ scanId: 1, photosFound: photos, unreadableEntries: 0, withoutExifDate: 0 }}
      busy={false}
      onChooseAnother={() => {}}
    />,
  );
}

describe('TimelineView', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it.each([
    [1, '1 seria (2 zdjęcia).'],
    [2, '2 serie (4 zdjęcia).'],
    [5, '5 serii (10 zdjęć).'],
    [12, '12 serii (24 zdjęcia).'],
    [22, '22 serie (44 zdjęcia).'],
  ])('counts %i series in Polish', async (count, text) => {
    mockedTimeline.mockResolvedValue(entries(2 * count));
    mockedSeries.mockResolvedValue(pairs(count));
    renderView(2 * count);

    expect(await screen.findByRole('status')).toHaveTextContent(text);
  });

  it('starts with the series view and switches to all photos', async () => {
    mockedTimeline.mockResolvedValue(entries(4));
    mockedSeries.mockResolvedValue(pairs(1));
    renderView(4);
    const seriesButton = screen.getByRole('button', { name: 'Serie' });
    const allButton = screen.getByRole('button', { name: 'Wszystkie zdjęcia' });
    expect(seriesButton).toHaveAttribute('aria-pressed', 'true');
    expect(allButton).toHaveAttribute('aria-pressed', 'false');

    await userEvent.click(allButton);

    expect(seriesButton).toHaveAttribute('aria-pressed', 'false');
    expect(allButton).toHaveAttribute('aria-pressed', 'true');
    expect(await screen.findByTestId('timeline')).toBeInTheDocument();
  });

  it('explains when there are no series and still offers all photos', async () => {
    mockedTimeline.mockResolvedValue(entries(3));
    mockedSeries.mockResolvedValue([]);
    renderView(3);

    expect(await screen.findByText('Nie znaleziono serii zdjęć.')).toBeInTheDocument();
    expect(screen.queryByTestId('timeline')).toBeNull();

    await userEvent.click(screen.getByRole('button', { name: 'Wszystkie zdjęcia' }));

    expect(await screen.findByTestId('timeline')).toBeInTheDocument();
    expect(screen.queryByText('Nie znaleziono serii zdjęć.')).toBeNull();
  });
});
