import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { getSeries, getTimeline, type Grouping, type Series, type TimelineEntry } from './api';
import '../../i18n';
import type { Completing } from '../scan/scanReducer';
import { sendViewerContext } from '../viewer/api';
import { loadGap, saveGap } from './gapSetting';
import { TimelineView } from './TimelineView';

vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  getTimeline: vi.fn(),
  getSeries: vi.fn(),
}));
vi.mock('../reject/api', async (importActual) => ({
  ...(await importActual<typeof import('../reject/api')>()),
  canUndo: vi.fn(() => Promise.resolve(false)),
  undoLast: vi.fn(),
  onTimelineChanged: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock('../accept/api', () => ({
  getAcceptedIds: vi.fn(() => Promise.resolve([])),
  onAcceptedChanged: vi.fn(() => Promise.resolve(() => {})),
  acceptPhoto: vi.fn(),
  unacceptPhoto: vi.fn(),
}));
vi.mock('../viewer/api', () => ({
  openViewer: vi.fn(() => Promise.resolve()),
  sendViewerContext: vi.fn(() => Promise.resolve()),
  onViewerShown: vi.fn(() => Promise.resolve(() => {})),
  onViewerClosed: vi.fn(() => Promise.resolve(() => {})),
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
    timeSource: 'exif',
    format: 'jpeg',
  }));
}

/** `count` series of two photos each. */
function pairs(count: number): Series[] {
  return Array.from({ length: count }, (_, i) => ({
    start: 2 * i,
    len: 2,
    label: { kind: 'plain' },
  }));
}

/** `series` of a timeline of `count` photos shown in timeline order. */
function grouping(series: Series[], count: number): Grouping {
  return { order: Array.from({ length: count }, (_, i) => i), series };
}

function renderView(photos: number, completing: Completing | null = null) {
  render(
    <TimelineView
      path="C:\Zdjęcia"
      summary={{
        scanId: 1,
        photosFound: photos,
        unreadableEntries: 0,
        withoutDate: 0,
        refreshing: false,
      }}
      completing={completing}
      busy={false}
      onChooseAnother={() => {}}
    />,
  );
}

describe('TimelineView', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
  });

  it('shows how far reading EXIF in the background has got', async () => {
    mockedTimeline.mockResolvedValue(entries(2));
    mockedSeries.mockResolvedValue(grouping([], 2));
    renderView(2, { done: 1234, total: 34600 });

    // pl-PL groups digits with a space from five digits on.
    expect(await screen.findByText(/Uzupełnianie dat z EXIF: 1234 z 34\s600\./)).toBeVisible();
    // The note floats over the grid, so the bar's summary line keeps its length.
    expect(screen.getByRole('status')).not.toHaveTextContent('Uzupełnianie dat');
  });

  it('gives the summary line its full text as a tooltip', async () => {
    mockedTimeline.mockResolvedValue(entries(2));
    mockedSeries.mockResolvedValue(grouping(pairs(1), 2));
    renderView(2);

    const status = await screen.findByRole('status');
    await vi.waitFor(() => expect(status).toHaveTextContent('1 seria (2 zdjęcia).'));
    expect(status).toHaveAttribute('title', status.textContent);
  });

  it('regroups with the gap from the slider and remembers it for the folder', async () => {
    saveGap('C:\\Zdjęcia', 4);
    mockedTimeline.mockResolvedValue(entries(6));
    mockedSeries.mockImplementation((gap) =>
      Promise.resolve(grouping(pairs(gap === 30 ? 3 : 1), 6)),
    );
    renderView(6);

    const slider = screen.getByRole('slider', { name: 'Odstęp w serii' });
    expect(slider).toHaveValue('4');
    expect(screen.getByText('4 s')).toBeInTheDocument();
    expect(await screen.findByRole('status')).toHaveTextContent('1 seria (2 zdjęcia).');
    expect(mockedSeries).toHaveBeenLastCalledWith(4);

    fireEvent.change(slider, { target: { value: '30' } });

    expect(screen.getByText('30 s')).toBeInTheDocument();
    expect(await screen.findByText(/3 serie \(6 zdjęć\)\./)).toBeInTheDocument();
    expect(mockedSeries).toHaveBeenLastCalledWith(30);
    expect(loadGap('C:\\Zdjęcia')).toBe(30);
  });

  it('steps the gap by one second with the buttons, within the range', async () => {
    saveGap('C:\\Zdjęcia', 2);
    mockedTimeline.mockResolvedValue(entries(2));
    mockedSeries.mockResolvedValue(grouping(pairs(1), 2));
    renderView(2);
    const slider = screen.getByRole('slider', { name: 'Odstęp w serii' });
    const decrease = screen.getByRole('button', { name: 'Zmniejsz odstęp' });
    const increase = screen.getByRole('button', { name: 'Zwiększ odstęp' });

    await userEvent.click(decrease);
    expect(slider).toHaveValue('1');
    expect(decrease).toBeDisabled();
    expect(loadGap('C:\\Zdjęcia')).toBe(1);

    await userEvent.click(increase);
    await userEvent.click(increase);
    expect(slider).toHaveValue('3');
    expect(decrease).toBeEnabled();
    expect(mockedSeries).toHaveBeenLastCalledWith(3);
  });

  it('cannot go above the largest gap', () => {
    saveGap('C:\\Zdjęcia', 60);
    mockedTimeline.mockResolvedValue(entries(2));
    mockedSeries.mockResolvedValue(grouping(pairs(1), 2));
    renderView(2);

    expect(screen.getByRole('button', { name: 'Zwiększ odstęp' })).toBeDisabled();
  });

  it('keeps an open viewer in step with the view and gap', async () => {
    mockedTimeline.mockResolvedValue(entries(4));
    mockedSeries.mockResolvedValue(grouping(pairs(1), 4));
    renderView(4);
    expect(sendViewerContext).toHaveBeenLastCalledWith({ mode: 'series', gap: 10 });

    await userEvent.click(screen.getByRole('button', { name: 'Wszystkie zdjęcia' }));
    expect(sendViewerContext).toHaveBeenLastCalledWith({ mode: 'all', gap: 10 });

    await userEvent.click(screen.getByRole('button', { name: 'Zmniejsz odstęp' }));
    expect(sendViewerContext).toHaveBeenLastCalledWith({ mode: 'all', gap: 9 });
  });

  it('starts other folders at the default gap', async () => {
    saveGap('D:\\Inne', 4);
    mockedTimeline.mockResolvedValue(entries(2));
    mockedSeries.mockResolvedValue(grouping(pairs(1), 2));
    renderView(2);

    expect(screen.getByRole('slider', { name: 'Odstęp w serii' })).toHaveValue('10');
    await screen.findByText(/1 seria/);
    expect(mockedSeries).toHaveBeenCalledWith(10);
  });

  it.each([
    [1, '1 seria (2 zdjęcia).'],
    [2, '2 serie (4 zdjęcia).'],
    [5, '5 serii (10 zdjęć).'],
    [12, '12 serii (24 zdjęcia).'],
    [22, '22 serie (44 zdjęcia).'],
  ])('counts %i series in Polish', async (count, text) => {
    mockedTimeline.mockResolvedValue(entries(2 * count));
    mockedSeries.mockResolvedValue(grouping(pairs(count), 2 * count));
    renderView(2 * count);

    expect(await screen.findByRole('status')).toHaveTextContent(text);
  });

  it('starts with the series view and switches to all photos', async () => {
    mockedTimeline.mockResolvedValue(entries(4));
    mockedSeries.mockResolvedValue(grouping(pairs(1), 4));
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
    mockedSeries.mockResolvedValue(grouping([], 3));
    renderView(3);

    expect(await screen.findByText('Nie znaleziono serii zdjęć.')).toBeInTheDocument();
    expect(screen.queryByTestId('timeline')).toBeNull();

    await userEvent.click(screen.getByRole('button', { name: 'Wszystkie zdjęcia' }));

    expect(await screen.findByTestId('timeline')).toBeInTheDocument();
    expect(screen.queryByText('Nie znaleziono serii zdjęć.')).toBeNull();
  });
});
