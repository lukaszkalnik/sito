import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import {
  checkForUpdate,
  installUpdate,
  onCheckRequested,
  UpdateError,
  type InstallProgress,
  type UpdateInfo,
} from './api';
import { UpdateNotice } from './UpdateNotice';
import '../../i18n';

vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  checkForUpdate: vi.fn(),
  installUpdate: vi.fn(),
  onCheckRequested: vi.fn(),
}));

const info: UpdateInfo = {
  version: '0.15.0',
  notes: '### New\n- Updates in the app',
  install: 'inApp',
};
const found = { currentVersion: '0.14.0', update: info };
const notFound = { currentVersion: '0.14.0', update: null };

/** Renders the notice and returns a function that picks „Sprawdź aktualizacje…”. */
async function renderNotice(): Promise<() => void> {
  let requestCheck = () => {};
  vi.mocked(onCheckRequested).mockImplementation((handler) => {
    requestCheck = handler;
    return Promise.resolve(() => {});
  });
  render(<UpdateNotice />);
  await waitFor(() => expect(onCheckRequested).toHaveBeenCalled());
  return () => act(() => requestCheck());
}

describe('UpdateNotice', () => {
  beforeEach(() => {
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('offers a newer version found at the start', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(found);
    await renderNotice();

    expect(await screen.findByText('Dostępna jest nowa wersja Sito: 0.15.0.')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Zainstaluj i uruchom ponownie' })).toBeEnabled();
  });

  it('shows nothing at the start without an update or when offline', async () => {
    vi.mocked(checkForUpdate).mockRejectedValueOnce(new UpdateError('network'));
    await renderNotice();

    await waitFor(() => expect(console.error).toHaveBeenCalled());
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });

  it('says from the menu that this is the newest version', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(notFound);
    const requestCheck = await renderNotice();

    requestCheck();

    expect(await screen.findByText('Masz najnowszą wersję (0.14.0).')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));
    expect(screen.queryByText(/najnowszą wersję/)).not.toBeInTheDocument();
  });

  it('says from the menu why the check failed', async () => {
    vi.mocked(checkForUpdate).mockResolvedValueOnce(notFound);
    const requestCheck = await renderNotice();
    vi.mocked(checkForUpdate).mockRejectedValueOnce(new UpdateError('network'));

    requestCheck();

    expect(
      await screen.findByText('Nie udało się sprawdzić aktualizacji. Brak połączenia z GitHubem.'),
    ).toBeInTheDocument();
  });

  it('shows the release notes in a dialog', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(found);
    await renderNotice();

    await userEvent.click(await screen.findByRole('button', { name: 'Co nowego?' }));

    const dialog = screen.getByRole('dialog', { name: 'Co nowego w wersji 0.15.0' });
    expect(dialog).toHaveTextContent('Updates in the app');
    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('hides the offer with „Nie teraz”', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(found);
    await renderNotice();

    await userEvent.click(await screen.findByRole('button', { name: 'Nie teraz' }));

    expect(screen.queryByText(/nowa wersja/)).not.toBeInTheDocument();
  });

  it('shows the download progress, then the install', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(found);
    let report: (progress: InstallProgress) => void = () => {};
    vi.mocked(installUpdate).mockImplementation((onProgress) => {
      report = onProgress;
      return new Promise(() => {});
    });
    await renderNotice();

    await userEvent.click(
      await screen.findByRole('button', { name: 'Zainstaluj i uruchom ponownie' }),
    );
    expect(screen.getByText('Pobieranie aktualizacji…')).toBeInTheDocument();

    act(() => report({ kind: 'downloading', downloaded: 45, total: 100 }));
    expect(screen.getByText('Pobieranie aktualizacji… 45%')).toBeInTheDocument();
    expect(screen.queryByRole('button')).not.toBeInTheDocument();

    act(() => report({ kind: 'installing' }));
    expect(screen.getByText('Instalowanie aktualizacji…')).toBeInTheDocument();
  });

  it('offers the update again when installing fails', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue(found);
    vi.mocked(installUpdate).mockRejectedValue(new UpdateError('badSignature'));
    await renderNotice();

    await userEvent.click(
      await screen.findByRole('button', { name: 'Zainstaluj i uruchom ponownie' }),
    );

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Nie udało się zainstalować aktualizacji. Pobrany plik nie ma prawidłowego podpisu Sito.',
    );
    expect(screen.getByRole('button', { name: 'Zainstaluj i uruchom ponownie' })).toBeEnabled();
  });

  it('points to the releases page where Sito cannot install it', async () => {
    vi.mocked(checkForUpdate).mockResolvedValue({
      ...found,
      update: { ...info, install: 'manual' },
    });
    await renderNotice();

    expect(
      await screen.findByText('Pobierz ją ze strony wydań Sito na GitHubie.'),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole('button', { name: 'Zainstaluj i uruchom ponownie' }),
    ).not.toBeInTheDocument();
  });
});
