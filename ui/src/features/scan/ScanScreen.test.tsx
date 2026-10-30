import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  onScanRefreshed,
  pickFolder,
  ScanError,
  scanFolder,
  type ScanProgress,
  type ScanSummary,
} from './api';
import { getSeries, getTimeline, type Series, type TimelineEntry } from '../timeline/api';
import '../../i18n';
import { ScanScreen } from './ScanScreen';

vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  pickFolder: vi.fn(),
  scanFolder: vi.fn(),
  onScanRefreshed: vi.fn(),
}));
vi.mock('../timeline/api', () => ({
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
}));

const mockedPick = vi.mocked(pickFolder);
const mockedScan = vi.mocked(scanFolder);
const mockedTimeline = vi.mocked(getTimeline);
const mockedSeries = vi.mocked(getSeries);

function summary(photosFound: number, extra: { unreadable?: number; withoutExif?: number } = {}) {
  return {
    scanId: 1,
    photosFound,
    unreadableEntries: extra.unreadable ?? 0,
    withoutDate: extra.withoutExif ?? 0,
    refreshing: false,
  };
}

const entries: TimelineEntry[] = [
  {
    id: 0,
    name: 'IMG_2.jpg',
    relativeDir: '',
    relativePath: 'IMG_2.jpg',
    takenAt: '2024-06-01T12:00:05',
    timeSource: 'exif',
    format: 'jpeg',
  },
  {
    id: 1,
    name: 'IMG_1.jpg',
    relativeDir: 'Camera',
    relativePath: 'Camera\\IMG_1.jpg',
    takenAt: '2024-06-01T12:00:00',
    timeSource: 'file',
    format: 'jpeg',
  },
];

/**
 * A timeline of `count` photos (repeating `entries`); the status line counts
 * the loaded timeline, since rejected photos leave it.
 */
function timelineOf(count: number): TimelineEntry[] {
  return Array.from({ length: count }, (_, id) => ({ ...(entries[id % 2] as TimelineEntry), id }));
}

describe('ScanScreen', () => {
  /** Announces a finished background refresh, like the backend. */
  let announceRefresh: (summary: ScanSummary) => void = () => {};

  beforeEach(() => {
    vi.resetAllMocks();
    localStorage.clear();
    mockedPick.mockResolvedValue('C:\\Zdjęcia');
    mockedTimeline.mockResolvedValue(entries);
    // Series of the first two photos, in the order of the timeline last answered.
    mockedSeries.mockImplementation(async () => {
      const timeline: TimelineEntry[] = (await mockedTimeline.mock.results.at(-1)?.value) ?? [];
      const series: Series[] = [{ start: 0, len: 2, label: { kind: 'plain' } }];
      return { order: timeline.map((_, i) => i), series };
    });
    vi.mocked(onScanRefreshed).mockImplementation((handler) => {
      announceRefresh = handler;
      return Promise.resolve(() => {});
    });
  });

  it('reopens the last folder on start, from the cache, and shows the refresh', async () => {
    localStorage.setItem('sito.lastFolder', 'D:\\Złote gody');
    mockedTimeline.mockResolvedValue(timelineOf(3));
    mockedScan.mockResolvedValue({ ...summary(3), refreshing: true });
    render(<ScanScreen />);

    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent(
        /Znaleziono 3 zdjęcia\..*Sprawdzanie zmian…/,
      ),
    );
    expect(mockedScan).toHaveBeenCalledWith('D:\\Złote gody', expect.any(Function));
    expect(mockedPick).not.toHaveBeenCalled();

    act(() => announceRefresh({ ...summary(3, { unreadable: 2 }), refreshing: false }));

    expect(await screen.findByRole('status')).toHaveTextContent(
      'Nie udało się odczytać 2 elementów.',
    );
    expect(screen.getByRole('status')).not.toHaveTextContent('Sprawdzanie zmian');
  });

  it('remembers the folder opened last', async () => {
    mockedScan.mockResolvedValue(summary(2));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }));
    await screen.findByTestId('timeline');

    expect(localStorage.getItem('sito.lastFolder')).toBe('C:\\Zdjęcia');
  });

  it.each([
    [1, 'Znaleziono 1 zdjęcie.'],
    [2, 'Znaleziono 2 zdjęcia.'],
    [5, 'Znaleziono 5 zdjęć.'],
    [22, 'Znaleziono 22 zdjęcia.'],
    // pl-PL groups digits only from 10 000 up; Testing Library turns the
    // no-break group separator into a plain space.
    [1234, 'Znaleziono 1234 zdjęcia.'],
    [12345, 'Znaleziono 12 345 zdjęć.'],
  ])('shows the timeline with the count for %i photos', async (count, text) => {
    mockedScan.mockResolvedValue(summary(count));
    mockedTimeline.mockResolvedValue(timelineOf(count));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }));

    expect(await screen.findByRole('status')).toHaveTextContent(text);
    expect(await screen.findByText('1 seria (2 zdjęcia).', { exact: false })).toBeInTheDocument();
    expect(screen.getByText('C:\\Zdjęcia')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Wybierz inny folder' })).toBeEnabled();
    expect(await screen.findByTestId('timeline')).toBeInTheDocument();
  });

  it('mentions photos without EXIF date and unreadable entries', async () => {
    mockedScan.mockResolvedValue(summary(3, { withoutExif: 2, unreadable: 5 }));
    mockedTimeline.mockResolvedValue(timelineOf(3));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    const status = await screen.findByText(/Znaleziono 3 zdjęcia\./);
    expect(status).toHaveTextContent('Bez daty zdjęcia: 2.');
    expect(status).toHaveTextContent('Nie udało się odczytać 5 elementów.');
  });

  it('shows a dedicated message when no photos were found', async () => {
    mockedScan.mockResolvedValue(summary(0));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    expect(await screen.findByText('Nie znaleziono żadnych zdjęć.')).toBeInTheDocument();
    expect(mockedTimeline).not.toHaveBeenCalled();
  });

  it('shows both progress phases and disables the button while scanning', async () => {
    let report: (p: ScanProgress) => void = () => {};
    mockedScan.mockImplementation((_path, onProgress) => {
      report = onProgress;
      return new Promise(() => {});
    });
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));
    report({ phase: 'searching', found: 15000 });
    expect(
      await screen.findByText('Wyszukiwanie zdjęć… znaleziono 15 000 zdjęć'),
    ).toBeInTheDocument();

    report({ phase: 'reading', done: 12000, total: 15000 });
    expect(await screen.findByText('Odczytywanie dat: 12 000 z 15 000')).toBeInTheDocument();
    expect(screen.getByRole('button')).toBeDisabled();
  });

  it('shows a loading error when the timeline cannot be fetched', async () => {
    mockedScan.mockResolvedValue(summary(2));
    mockedTimeline.mockRejectedValue(new Error('boom'));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    expect(await screen.findByRole('alert')).toHaveTextContent('Nie udało się wczytać osi czasu.');
  });

  it.each([
    ['notFound', 'Nie znaleziono wybranego folderu.'],
    ['notADirectory', 'Wybrana ścieżka nie jest folderem.'],
    ['io', 'Nie udało się odczytać wybranego folderu.'],
    ['internal', 'Wystąpił nieoczekiwany błąd. Spróbuj ponownie.'],
  ] as const)('shows a Polish message for error %s', async (code, text) => {
    mockedScan.mockRejectedValue(new ScanError(code));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    expect(await screen.findByRole('alert')).toHaveTextContent(text);
  });

  it('stays idle when the folder dialog is cancelled', async () => {
    mockedPick.mockResolvedValue(null);
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }));

    expect(mockedScan).not.toHaveBeenCalled();
    expect(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' })).toBeEnabled();
  });
});
