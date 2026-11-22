import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import i18n from '../../i18n';
import {
  CacheError,
  cancelCacheMove,
  checkCacheFolder,
  clearPhotoData,
  clearThumbnails,
  getCacheSettings,
  measureCache,
  moveCache,
  onShowCacheSettings,
  pickCacheFolder,
  revealCacheFolder,
  setThumbnailLimit,
  type CacheSettings,
  type MoveProgress,
} from './api';
import type { BackgroundWork } from './backgroundWork';
import { CacheSettingsPanel } from './CacheSettingsPanel';

vi.mock('./api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('./api')>()),
  getCacheSettings: vi.fn(),
  measureCache: vi.fn(),
  setThumbnailLimit: vi.fn(),
  clearThumbnails: vi.fn(),
  clearPhotoData: vi.fn(),
  revealCacheFolder: vi.fn(),
  checkCacheFolder: vi.fn(),
  moveCache: vi.fn(),
  cancelCacheMove: vi.fn(),
  pickCacheFolder: vi.fn(),
  onShowCacheSettings: vi.fn(),
}));

const MB = 1024 * 1024;

const windows: CacheSettings = {
  folder: 'C:\\Users\\Ala\\AppData\\Local\\io.github.lukaszkalnik.sito',
  location: { kind: 'default' },
  platform: 'windows',
  thumbnailLimitMb: 500,
};

/** Renders the panel and opens the dialog as the menu item does. */
async function openDialog(settings: CacheSettings = windows, work: BackgroundWork = 'none') {
  vi.mocked(getCacheSettings).mockResolvedValue(settings);
  let show = () => {};
  vi.mocked(onShowCacheSettings).mockImplementation((handler) => {
    show = handler;
    return Promise.resolve(() => {});
  });
  const view = render(<CacheSettingsPanel work={work} />);
  await waitFor(() => expect(onShowCacheSettings).toHaveBeenCalled());
  act(() => show());
  const dialog = await screen.findByRole('dialog', { name: 'Pamięć podręczna' });
  await within(dialog).findByLabelText('Miejsce na miniatury');
  return { dialog, view };
}

/** Text with no-break spaces as plain ones. */
function plain(text: string | null): string {
  return (text ?? '').replace(/\u00a0/gu, ' ');
}

describe('CacheSettingsPanel', () => {
  beforeEach(() => {
    vi.mocked(measureCache).mockResolvedValue({ thumbnails: 412 * MB, photoData: 18 * MB });
    vi.mocked(setThumbnailLimit).mockImplementation((mb) => Promise.resolve(Math.max(50, mb)));
    vi.mocked(clearThumbnails).mockResolvedValue();
    vi.mocked(clearPhotoData).mockResolvedValue();
    vi.mocked(revealCacheFolder).mockResolvedValue();
    vi.mocked(cancelCacheMove).mockResolvedValue();
    vi.mocked(pickCacheFolder).mockResolvedValue('D:\\Cache');
    vi.mocked(checkCacheFolder).mockResolvedValue({
      target: 'D:\\Cache\\Sito-cache',
      network: false,
      size: 430 * MB,
    });
  });

  afterEach(async () => {
    vi.resetAllMocks();
    await i18n.changeLanguage('pl');
  });

  it('stays closed until the menu item is picked', async () => {
    vi.mocked(getCacheSettings).mockResolvedValue(windows);
    vi.mocked(onShowCacheSettings).mockResolvedValue(() => {});
    render(<CacheSettingsPanel work="none" />);
    await waitFor(() => expect(getCacheSettings).toHaveBeenCalled());

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    expect(measureCache).not.toHaveBeenCalled();
  });

  it('shows the limit and the space in use once measured', async () => {
    let measured: (size: { thumbnails: number; photoData: number }) => void = () => {};
    vi.mocked(measureCache).mockReturnValue(new Promise((resolve) => (measured = resolve)));
    const { dialog } = await openDialog();

    expect(within(dialog).getByLabelText('Miejsce na miniatury')).toHaveValue(500);
    expect(within(dialog).getByRole('status')).toHaveTextContent('Obliczanie zajętego miejsca…');
    act(() => measured({ thumbnails: 412 * MB, photoData: 18 * MB }));

    expect(await within(dialog).findByText('Miniatury: 412 MB · Dane zdjęć: 18 MB')).toBeVisible();
  });

  it.each([
    ['en', 'Thumbnails: 1,536 MB · Photo data: 900 kB', 'Cache'],
    ['de', 'Vorschaubilder: 1.536 MB · Fotodaten: 900 kB', 'Zwischenspeicher'],
    ['pl', 'Miniatury: 1536 MB · Dane zdjęć: 900 kB', 'Pamięć podręczna'],
  ])('shows sizes as %s formats them', async (language, usage, title) => {
    await act(() => i18n.changeLanguage(language));
    vi.mocked(measureCache).mockResolvedValue({ thumbnails: 1536 * MB, photoData: 900 * 1024 });
    vi.mocked(getCacheSettings).mockResolvedValue(windows);
    let show = () => {};
    vi.mocked(onShowCacheSettings).mockImplementation((handler) => {
      show = handler;
      return Promise.resolve(() => {});
    });
    render(<CacheSettingsPanel work="none" />);
    await waitFor(() => expect(onShowCacheSettings).toHaveBeenCalled());
    act(() => show());

    const dialog = await screen.findByRole('dialog', { name: title });
    await waitFor(() => expect(plain(dialog.textContent)).toContain(usage));
  });

  it('applies a typed limit when the field is left, at least 50 MB', async () => {
    const user = userEvent.setup();
    const { dialog } = await openDialog();
    const input = within(dialog).getByLabelText('Miejsce na miniatury');

    await user.clear(input);
    await user.type(input, '30');
    expect(setThumbnailLimit).not.toHaveBeenCalled();
    await user.tab();

    expect(setThumbnailLimit).toHaveBeenCalledWith(50);
    await waitFor(() => expect(input).toHaveValue(50));
  });

  it('applies a typed limit on Enter and measures again', async () => {
    const user = userEvent.setup();
    const { dialog } = await openDialog();
    const input = within(dialog).getByLabelText('Miejsce na miniatury');

    await user.clear(input);
    await user.type(input, '800{Enter}');

    expect(setThumbnailLimit).toHaveBeenCalledWith(800);
    await waitFor(() => expect(measureCache).toHaveBeenCalledTimes(2));
  });

  it('empties the thumbnails only once confirmed', async () => {
    const user = userEvent.setup();
    const { dialog } = await openDialog();

    await user.click(within(dialog).getByRole('button', { name: 'Wyczyść miniatury' }));
    const question = within(dialog).getByRole('alertdialog');
    expect(question).toHaveTextContent('Usunąć wszystkie miniatury z pamięci podręcznej?');
    await user.click(within(question).getByRole('button', { name: 'Anuluj' }));
    expect(clearThumbnails).not.toHaveBeenCalled();

    await user.click(within(dialog).getByRole('button', { name: 'Wyczyść miniatury' }));
    await user.click(within(dialog).getByRole('button', { name: 'Usuń' }));

    expect(clearThumbnails).toHaveBeenCalledTimes(1);
    await waitFor(() => expect(measureCache).toHaveBeenCalledTimes(2));
  });

  it('empties the photo data once its cost is confirmed', async () => {
    const user = userEvent.setup();
    const { dialog } = await openDialog();

    await user.click(within(dialog).getByRole('button', { name: 'Wyczyść dane zdjęć' }));
    expect(within(dialog).getByRole('alertdialog')).toHaveTextContent(
      'może to potrwać kilka minut',
    );
    await user.click(within(dialog).getByRole('button', { name: 'Usuń' }));

    expect(clearPhotoData).toHaveBeenCalledTimes(1);
  });

  it.each<BackgroundWork>(['scanning', 'exifPass'])(
    'keeps the photo data while the folder is read (%s)',
    async (work) => {
      const { dialog } = await openDialog(windows, work);

      expect(within(dialog).getByRole('button', { name: 'Wyczyść dane zdjęć' })).toBeDisabled();
      expect(within(dialog).getByRole('button', { name: 'Wyczyść miniatury' })).toBeEnabled();
    },
  );

  it('keeps the cache in place while a folder is scanned, but not during the EXIF pass', async () => {
    const { dialog, view } = await openDialog(windows, 'scanning');
    expect(within(dialog).getByRole('button', { name: 'Zmień…' })).toBeDisabled();

    view.rerender(<CacheSettingsPanel work="exifPass" />);
    expect(within(dialog).getByRole('button', { name: 'Zmień…' })).toBeEnabled();
  });

  it('says why emptying failed', async () => {
    const user = userEvent.setup();
    vi.mocked(clearPhotoData).mockRejectedValue(new CacheError('busy'));
    const { dialog } = await openDialog();

    await user.click(within(dialog).getByRole('button', { name: 'Wyczyść dane zdjęć' }));
    await user.click(within(dialog).getByRole('button', { name: 'Usuń' }));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent('Trwa wczytywanie folderu');
  });

  it.each([
    ['macos', 'Pokaż w Finderze'],
    ['other', 'Pokaż w menedżerze plików'],
  ] as const)(
    'on %s shows the folder in the file manager instead of moving it',
    async (platform, label) => {
      const user = userEvent.setup();
      const { dialog } = await openDialog({
        ...windows,
        folder: '/Users/ala/Library/Caches/sito',
        platform,
      });

      expect(within(dialog).queryByRole('button', { name: 'Zmień…' })).not.toBeInTheDocument();
      expect(within(dialog).getByText('Folder pamięci podręcznej')).toBeVisible();
      await user.click(within(dialog).getByRole('button', { name: label }));

      expect(revealCacheFolder).toHaveBeenCalledTimes(1);
    },
  );

  it('closes with „Zamknij”', async () => {
    const user = userEvent.setup();
    const { dialog } = await openDialog();

    await user.click(within(dialog).getByRole('button', { name: 'Zamknij' }));

    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });
});

describe('moving the cache', () => {
  let progress: (progress: MoveProgress) => void;
  let finish: (settings: CacheSettings) => void;
  let fail: (err: CacheError) => void;

  beforeEach(() => {
    vi.mocked(measureCache).mockResolvedValue({ thumbnails: 412 * MB, photoData: 18 * MB });
    vi.mocked(cancelCacheMove).mockResolvedValue();
    vi.mocked(pickCacheFolder).mockResolvedValue('D:\\Cache');
    vi.mocked(checkCacheFolder).mockResolvedValue({
      target: 'D:\\Cache\\Sito-cache',
      network: false,
      size: 430 * MB,
    });
    vi.mocked(moveCache).mockImplementation(
      (_folder, onProgress) =>
        new Promise((resolve, reject) => {
          progress = onProgress;
          finish = resolve;
          fail = reject;
        }),
    );
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  async function chooseFolder(settings: CacheSettings = windows) {
    const user = userEvent.setup();
    const { dialog } = await openDialog(settings);
    await user.click(within(dialog).getByRole('button', { name: 'Zmień…' }));
    return { user, dialog };
  }

  it('asks with the size and the target, then moves with progress', async () => {
    const { user, dialog } = await chooseFolder();

    const question = await within(dialog).findByRole('alertdialog');
    expect(question).toHaveTextContent(
      'Przenieść pamięć podręczną (430 MB) do D:\\Cache\\Sito-cache?',
    );
    expect(checkCacheFolder).toHaveBeenCalledWith('D:\\Cache');
    await user.click(within(question).getByRole('button', { name: 'Przenieś' }));

    expect(moveCache).toHaveBeenCalledWith('D:\\Cache', expect.any(Function));
    expect(within(dialog).getByRole('status')).toHaveTextContent('Przenoszenie…');
    expect(within(dialog).getByRole('button', { name: 'Zamknij' })).toBeDisabled();
    act(() => progress({ done: 1234, total: 41000 }));
    expect(plain(within(dialog).getByRole('status').textContent)).toBe(
      'Przenoszenie: 1234 z 41 000 plików',
    );
    act(() => progress({ done: 1, total: 1 }));
    expect(within(dialog).getByRole('status')).toHaveTextContent('Przenoszenie: 1 z 1 pliku');

    act(() =>
      finish({ ...windows, folder: 'D:\\Cache\\Sito-cache', location: { kind: 'chosen' } }),
    );
    expect(await within(dialog).findByText('D:\\Cache\\')).toBeVisible();
    expect(within(dialog).getByRole('button', { name: 'Przywróć domyślny' })).toBeEnabled();
  });

  it('warns about a network drive', async () => {
    vi.mocked(checkCacheFolder).mockResolvedValue({
      target: '\\\\nas\\cache\\Sito-cache',
      network: true,
      size: 10 * MB,
    });
    const { dialog } = await chooseFolder();

    expect(await within(dialog).findByRole('alertdialog')).toHaveTextContent(
      'Pamięć podręczna na dysku sieciowym może spowolnić Sito.',
    );
  });

  it('moves nothing when the question is answered with „Anuluj”', async () => {
    const { user, dialog } = await chooseFolder();

    await user.click(await within(dialog).findByRole('button', { name: 'Anuluj' }));

    expect(moveCache).not.toHaveBeenCalled();
    expect(within(dialog).queryByRole('alertdialog')).not.toBeInTheDocument();
  });

  it('moves nothing when no folder is picked', async () => {
    vi.mocked(pickCacheFolder).mockResolvedValue(null);
    const { dialog } = await chooseFolder();

    await waitFor(() =>
      expect(within(dialog).getByRole('button', { name: 'Zmień…' })).toBeEnabled(),
    );
    expect(checkCacheFolder).not.toHaveBeenCalled();
  });

  it.each([
    ['insideOpenFolder', 'leży w folderze otwartym w Sito'],
    ['insideCache', 'To jest folder pamięci podręcznej'],
    ['notWritable', 'nie można zapisywać plików'],
    ['link', 'jest dowiązaniem'],
  ] as const)('refuses a folder that is %s', async (code, message) => {
    vi.mocked(checkCacheFolder).mockRejectedValue(new CacheError(code));
    const { dialog } = await chooseFolder();

    expect(await within(dialog).findByRole('alert')).toHaveTextContent(message);
    expect(moveCache).not.toHaveBeenCalled();
  });

  it('stops a copy with „Anuluj” and keeps the old folder', async () => {
    const { user, dialog } = await chooseFolder();
    await user.click(await within(dialog).findByRole('button', { name: 'Przenieś' }));
    act(() => progress({ done: 2, total: 10 }));

    await user.click(within(dialog).getByRole('button', { name: 'Anuluj' }));
    expect(cancelCacheMove).toHaveBeenCalledTimes(1);
    act(() => fail(new CacheError('cancelled')));

    expect(await within(dialog).findByText(/Przenoszenie anulowane/u)).toHaveAttribute(
      'role',
      'status',
    );
    expect(within(dialog).getByText('io.github.lukaszkalnik.sito')).toBeVisible();
  });

  it('says why a copy failed', async () => {
    const { user, dialog } = await chooseFolder();
    await user.click(await within(dialog).findByRole('button', { name: 'Przenieś' }));
    act(() => fail(new CacheError('copyDiffers')));

    expect(await within(dialog).findByRole('alert')).toHaveTextContent(
      'Kopia pamięci podręcznej różni się od oryginału.',
    );
  });

  it('moves back to the default folder after asking', async () => {
    const user = userEvent.setup();
    const chosen: CacheSettings = {
      ...windows,
      folder: 'D:\\Cache\\Sito-cache',
      location: { kind: 'chosen' },
    };
    const { dialog } = await openDialog(chosen);

    await user.click(within(dialog).getByRole('button', { name: 'Przywróć domyślny' }));
    expect(within(dialog).getByRole('alertdialog')).toHaveTextContent(
      'Przenieść pamięć podręczną z powrotem do folderu domyślnego?',
    );
    await user.click(within(dialog).getByRole('button', { name: 'Przenieś' }));

    expect(moveCache).toHaveBeenCalledWith(null, expect.any(Function));
    act(() => finish(windows));
    await waitFor(() =>
      expect(
        within(dialog).queryByRole('button', { name: 'Przywróć domyślny' }),
      ).not.toBeInTheDocument(),
    );
  });
});

describe('a missing cache folder', () => {
  const fallback: CacheSettings = {
    ...windows,
    location: { kind: 'fallback', chosen: 'D:\\Cache\\Sito-cache' },
  };

  beforeEach(() => {
    vi.mocked(getCacheSettings).mockResolvedValue(fallback);
    vi.mocked(measureCache).mockResolvedValue({ thumbnails: 0, photoData: 0 });
    vi.mocked(moveCache).mockResolvedValue(windows);
  });

  afterEach(() => {
    vi.resetAllMocks();
  });

  it('is said at start, and the notice opens the settings', async () => {
    const user = userEvent.setup();
    let show = () => {};
    vi.mocked(onShowCacheSettings).mockImplementation((handler) => {
      show = handler;
      return Promise.resolve(() => {});
    });
    render(<CacheSettingsPanel work="none" />);

    expect(await screen.findByRole('status')).toHaveTextContent(
      'Folder pamięci podręcznej D:\\Cache\\Sito-cache jest niedostępny – Sito używa folderu domyślnego.',
    );
    await user.click(screen.getByRole('button', { name: 'Ustawienia…' }));

    expect(await screen.findByRole('dialog', { name: 'Pamięć podręczna' })).toBeVisible();
    expect(show).toBeInstanceOf(Function);
  });

  it('can be closed', async () => {
    const user = userEvent.setup();
    vi.mocked(onShowCacheSettings).mockResolvedValue(() => {});
    render(<CacheSettingsPanel work="none" />);

    await user.click(await screen.findByRole('button', { name: 'Zamknij' }));

    expect(screen.queryByText(/jest niedostępny/u)).not.toBeInTheDocument();
  });

  it('is left for the default folder without moving anything or asking', async () => {
    const user = userEvent.setup();
    const { dialog } = await openDialog(fallback);

    await user.click(within(dialog).getByRole('button', { name: 'Przywróć domyślny' }));

    expect(moveCache).toHaveBeenCalledWith(null, expect.any(Function));
    expect(within(dialog).queryByRole('alertdialog')).not.toBeInTheDocument();
  });
});
