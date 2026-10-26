import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { pickFolder, ScanError, scanFolder, type ScanProgress } from '../../api/scan';
import '../../i18n';
import { ScanScreen } from './ScanScreen';

vi.mock('../../api/scan', async (importActual) => ({
  ...(await importActual<typeof import('../../api/scan')>()),
  pickFolder: vi.fn(),
  scanFolder: vi.fn(),
}));

const mockedPick = vi.mocked(pickFolder);
const mockedScan = vi.mocked(scanFolder);

function summary(photosFound: number, unreadableEntries = 0) {
  return { photosFound, entriesSeen: photosFound * 2, unreadableEntries };
}

describe('ScanScreen', () => {
  beforeEach(() => {
    vi.resetAllMocks();
    mockedPick.mockResolvedValue('C:\\Zdjęcia');
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
  ])('shows the result for %i photos', async (count, text) => {
    mockedScan.mockResolvedValue(summary(count));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button', { name: 'Wybierz folder ze zdjęciami' }));

    expect(await screen.findByText(text)).toBeInTheDocument();
    expect(screen.getByText('Folder: C:\\Zdjęcia')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Wybierz inny folder' })).toBeEnabled();
  });

  it('shows a dedicated message when no photos were found', async () => {
    mockedScan.mockResolvedValue(summary(0));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    expect(await screen.findByText('Nie znaleziono żadnych zdjęć.')).toBeInTheDocument();
  });

  it('mentions unreadable entries', async () => {
    mockedScan.mockResolvedValue(summary(3, 2));
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));

    expect(
      await screen.findByText('Nie udało się odczytać 2 elementów (brak dostępu).'),
    ).toBeInTheDocument();
  });

  it('shows progress and disables the button while scanning', async () => {
    let report: (p: ScanProgress) => void = () => {};
    mockedScan.mockImplementation((_path, onProgress) => {
      report = onProgress;
      return new Promise(() => {});
    });
    render(<ScanScreen />);

    await userEvent.click(screen.getByRole('button'));
    report({ entriesSeen: 15000, photosFound: 5 });

    expect(
      await screen.findByText('Przeszukiwanie… przejrzano 15 000 elementów, znaleziono 5 zdjęć'),
    ).toBeInTheDocument();
    expect(screen.getByRole('button')).toBeDisabled();
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
