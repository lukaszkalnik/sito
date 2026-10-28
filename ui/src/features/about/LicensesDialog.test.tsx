import { act, render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import App from '../../App';
import { appVersion, loadNotices, onShowLicenses } from './api';
import '../../i18n';

vi.mock('./api', () => ({
  onShowLicenses: vi.fn(),
  loadNotices: vi.fn(),
  appVersion: vi.fn(),
}));

/** Renders the app and returns a function that clicks „Licencje…” in the menu. */
async function renderApp(): Promise<() => void> {
  let showLicenses = () => {};
  vi.mocked(onShowLicenses).mockImplementation((handler) => {
    showLicenses = handler;
    return Promise.resolve(() => {});
  });
  render(<App />);
  await waitFor(() => expect(onShowLicenses).toHaveBeenCalled());
  return () => act(() => showLicenses());
}

describe('LicensesDialog', () => {
  beforeEach(() => {
    vi.mocked(appVersion).mockResolvedValue('0.9.0');
    vi.mocked(loadNotices).mockResolvedValue('THIRD-PARTY NOTICES\nreact 19.3.1 (MIT)');
    vi.spyOn(console, 'error').mockImplementation(() => {});
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('stays closed until the menu item is picked', async () => {
    await renderApp();

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(loadNotices).not.toHaveBeenCalled();
  });

  it('shows the version and the notices from the menu', async () => {
    const showLicenses = await renderApp();

    showLicenses();

    const dialog = await screen.findByRole('dialog', { name: 'Licencje' });
    expect(dialog).toHaveTextContent('Sito 0.9.0 jest dostępne na licencji MIT lub Apache 2.0.');
    expect(await screen.findByLabelText('Treść licencji')).toHaveTextContent('react 19.3.1 (MIT)');
  });

  it('closes with „Zamknij” and keeps the loaded notices for the next time', async () => {
    const showLicenses = await renderApp();
    showLicenses();
    await screen.findByText(/react 19.3.1/);

    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();

    showLicenses();
    expect(await screen.findByRole('dialog')).toHaveTextContent('react 19.3.1');
    expect(loadNotices).toHaveBeenCalledTimes(1);
  });

  it('reports a failed load and retries when opened again', async () => {
    vi.mocked(loadNotices).mockRejectedValueOnce(new Error('missing'));
    const showLicenses = await renderApp();
    showLicenses();

    expect(await screen.findByRole('alert')).toHaveTextContent('Nie udało się wczytać licencji.');

    await userEvent.click(screen.getByRole('button', { name: 'Zamknij' }));
    showLicenses();
    expect(await screen.findByText(/react 19.3.1/)).toBeInTheDocument();
  });
});
