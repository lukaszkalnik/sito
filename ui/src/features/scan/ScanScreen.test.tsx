import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import {
  getTimeline,
  pickFolder,
  ScanError,
  scanFolder,
  type ScanProgress,
  type TimelineEntry,
} from '../../api/scan';
import '../../i18n';
import { ScanScreen } from './ScanScreen';

vi.mock('../../api/scan', async (importActual) => ({
  ...(await importActual<typeof import('../../api/scan')>()),
  pickFolder: vi.fn(),
  scanFolder: vi.fn(),
  getTimeline: vi.fn(),
}));

const mockedPick = vi.mocked(pickFolder);
const mockedScan = vi.mocked(scanFolder);
const mockedTimeline = vi.mocked(getTimeline);

function summary(photosFound: number, extra: { unreadable?: number; withoutExif?: number } = {}) {
  return {
    photosFound,
    unreadableEntries: extra.unreadable ?? 0,
    withoutExifDate: extra.withoutExif ?? 0,
  };
}

const entries: TimelineEntry[] = [
  {
    id: 0,
    name: 'IMG_2.jpg',
    relativeDir: '',
    takenAt: '2024-06-01T12:00:05',
    fromFileDate: false,
  },
  {
    id: 1,
    name: 'IMG_1.jpg',
    relativeDir: 'Camera',
    takenAt: '2024-06-01T12:00:00',
    fromFileDate: true,
  },
];

describe('ScanScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockedPick.mockResolvedValue('C:\\Zdjęcia');
    mockedTimeline.mockResolvedValue(entries);
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
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }));

    expect(await screen.findByText(text)).toBeInTheDocument();
    expect(screen.getByText('C:\\Zdjęcia')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Wybierz inny folder' })).toBeEnabled();
    expect(await screen.findByTestId('timeline')).toBeInTheDocument();
  });

  it('mentions photos without EXIF date and unreadable entries', async () => {
    mockedScan.mockResolvedValue(summary(3, { withoutExif: 2, unreadable: 5 }));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    const status = await screen.findByText(/Znaleziono 3 zdjęcia\./);
    expect(status).toHaveTextContent('Bez daty EXIF: 2.');
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
