import { act, fireEvent, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { onTimelineChanged, rejectPhoto, undoState } from '../reject/api';
import { editSeries } from '../series-edit/api';
import {
  closeWindow,
  currentViewerRequest,
  getViewerPhoto,
  onViewerContext,
  onViewerShow,
  sendViewerShown,
  type ViewerPhoto,
} from './api';
import { pageOf } from './photoViewerFixtures';
import { ViewerApp } from './ViewerApp';

vi.mock('../reject/api', async (importActual) => ({
  ...(await importActual<typeof import('../reject/api')>()),
  undoState: vi.fn(),
  undo: vi.fn(),
  onTimelineChanged: vi.fn(),
  rejectPhoto: vi.fn(),
}));
vi.mock('../series-edit/api', async (importActual) => ({
  ...(await importActual<typeof import('../series-edit/api')>()),
  editSeries: vi.fn(),
}));
vi.mock('../timeline/thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));
vi.mock('../scan/api', () => ({ refreshFolder: vi.fn(() => Promise.resolve()) }));
vi.mock('../accept/api', () => ({
  getAcceptedIds: vi.fn(() => Promise.resolve([])),
  onAcceptedChanged: vi.fn(() => Promise.resolve(() => {})),
  acceptPhoto: vi.fn(),
  unacceptPhoto: vi.fn(),
}));
vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  closeWindow: vi.fn(),
  currentViewerRequest: vi.fn(),
  getViewerPhoto: vi.fn(),
  onViewerShow: vi.fn(),
  onViewerContext: vi.fn(),
  sendViewerShown: vi.fn(),
  photoUrl: (scanId: number, id: number) => `photo://localhost/${id}?scan=${scanId}`,
  setWindowTitle: vi.fn(() => Promise.resolve()),
}));

/** Pages of the photos 0–5 (photos 1–3 a series), each changed by `changes`. */
function serve(changes: (id: number) => Partial<ViewerPhoto> = () => ({})) {
  vi.mocked(getViewerPhoto).mockImplementation((request) =>
    Promise.resolve({ kind: 'photo', photo: pageOf(request.id, changes(request.id)) }),
  );
}

/** Opens the viewer on photo `id` and waits for it. */
async function openOn(id: number) {
  vi.mocked(currentViewerRequest).mockResolvedValue({ id, scanId: 1, mode: 'all', gap: 10 });
  render(<ViewerApp />);
  await screen.findByText(`IMG_${id}.jpg`, { selector: '.viewer-name' });
}

/** Steps the open viewer to photo `id` with the previous button. */
async function openOnAgain(id: number) {
  await userEvent.click(screen.getByRole('button', { name: 'Poprzednie zdjęcie' }));
  await screen.findByText(`IMG_${id}.jpg`, { selector: '.viewer-name' });
}

const shownName = () => document.querySelector('.viewer-name')?.textContent;
const editButton = () => screen.getByRole('button', { name: 'Edytuj serię' });
const menuItems = () =>
  within(screen.getByRole('menu'))
    .getAllByRole('menuitem')
    .map((item) => item.textContent);
const strip = () => screen.getByRole('navigation', { name: 'Zdjęcia w serii' });

function press(key: string, init: KeyboardEventInit = {}) {
  fireEvent.keyDown(window, { key, ...init });
}

describe('editing series in the viewer', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetAllMocks();
    const noUnlisten = () => Promise.resolve(() => {});
    vi.mocked(onViewerShow).mockImplementation(noUnlisten);
    vi.mocked(onViewerContext).mockImplementation(noUnlisten);
    vi.mocked(onTimelineChanged).mockImplementation(noUnlisten);
    vi.mocked(undoState).mockResolvedValue('none');
    vi.mocked(sendViewerShown).mockResolvedValue();
    vi.mocked(closeWindow).mockResolvedValue();
    vi.mocked(editSeries).mockImplementation((edit) =>
      Promise.resolve({ name: `IMG_${edit.photo}.jpg`, changed: [edit.photo] }),
    );
    serve();
  });

  it('opens the menu of the shown photo from the bar, with the keys of the edits', async () => {
    await openOn(2);

    await userEvent.click(editButton());

    expect(menuItems()).toEqual([
      'Podziel serię przed tym zdjęciemP',
      'Wyjmij z seriiW',
      'Rozdziel serię',
    ]);
    expect(
      screen.getByRole('menuitem', { name: 'Podziel serię przed tym zdjęciem' }),
    ).toHaveFocus();
  });

  it('disables the bar button where there is nothing to edit', async () => {
    serve((id) => (id === 1 ? { series: [pageOf(1).entry] } : {}));
    await openOn(0);
    expect(editButton()).toBeDisabled();

    await userEvent.click(screen.getByRole('button', { name: 'Następne zdjęcie' }));
    await screen.findByText('IMG_1.jpg', { selector: '.viewer-name' });

    // A series of one.
    expect(editButton()).toBeDisabled();
  });

  it('offers only restoring on a photo taken out by hand, marked after its name', async () => {
    serve((id) => ({ pinned: id === 0 }));
    await openOn(0);

    expect(screen.getByRole('img', { name: /Wyjęte z serii ręcznie/ })).toBeInTheDocument();
    await userEvent.click(editButton());
    await userEvent.click(
      screen.getByRole('menuitem', { name: 'Przywróć automatyczne grupowanie' }),
    );

    expect(editSeries).toHaveBeenCalledWith({ kind: 'restore', photo: 0 }, 10);
    expect(await screen.findByText('Przywrócono automatyczne grupowanie')).toBeInTheDocument();
  });

  it('marks an edited series in the strip and pulses what a restore changed', async () => {
    serve((id) => ({ seriesEdited: id >= 1 && id <= 3 }));
    vi.mocked(editSeries).mockResolvedValue({ name: 'IMG_1.jpg', changed: [3, 7] });
    await openOn(2);

    expect(
      within(strip()).getByRole('img', { name: /Seria edytowana ręcznie/ }),
    ).toBeInTheDocument();
    await userEvent.click(editButton());
    await userEvent.click(
      screen.getByRole('menuitem', { name: 'Przywróć automatyczne grupowanie' }),
    );

    expect(editSeries).toHaveBeenCalledWith({ kind: 'restore', photo: 1 }, 10);
    const items = within(strip()).getAllByRole('button');
    await vi.waitFor(() =>
      expect(items.map((item) => item.classList.contains('viewer-strip-item--pulsed'))).toEqual([
        false,
        false,
        true,
      ]),
    );
    expect(shownName()).toBe('IMG_2.jpg');
  });

  it('opens the menu of another strip photo on a right-click, keeping the shown photo', async () => {
    await openOn(1);

    fireEvent.contextMenu(within(strip()).getByRole('button', { name: 'IMG_2.jpg' }), {
      clientX: 40,
      clientY: 50,
    });
    await userEvent.click(
      screen.getByRole('menuitem', { name: 'Podziel serię przed tym zdjęciem' }),
    );

    expect(editSeries).toHaveBeenCalledWith({ kind: 'cut', photo: 2 }, 10);
    expect(await screen.findByText('Podzielono serię')).toBeInTheDocument();
    expect(shownName()).toBe('IMG_1.jpg');
  });

  it('opens the menu of the shown photo on a right-click on it', async () => {
    await openOn(3);

    const stage = document.querySelector('.viewer-stage');
    if (!stage) throw new Error('no photo stage');
    fireEvent.contextMenu(stage, { clientX: 5, clientY: 5 });

    expect(menuItems()).toEqual(['Wyjmij z seriiW', 'Rozdziel serię']);
  });

  it('opens the menu with Shift+F10 on a focused strip photo, else of the shown photo', async () => {
    await openOn(2);

    within(strip()).getByRole('button', { name: 'IMG_1.jpg' }).focus();
    press('F10', { shiftKey: true });
    expect(menuItems()).toEqual(['Wyjmij z seriiW', 'Rozdziel serię']);
    press('Escape');
    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(within(strip()).getByRole('button', { name: 'IMG_1.jpg' })).toHaveFocus();

    act(() => (document.activeElement as HTMLElement).blur());
    press('F10', { shiftKey: true });
    expect(menuItems()).toEqual([
      'Podziel serię przed tym zdjęciemP',
      'Wyjmij z seriiW',
      'Rozdziel serię',
    ]);
  });

  it('cuts with P and takes out with W, staying in the series', async () => {
    await openOn(2);

    press('p');
    expect(await screen.findByText('Podzielono serię')).toBeInTheDocument();
    expect(editSeries).toHaveBeenCalledWith({ kind: 'cut', photo: 2 }, 10);
    // The cut is done once its toast is up; a key pressed before then is ignored.
    await act(async () => {});
    press('W', { shiftKey: true });

    await vi.waitFor(() =>
      expect(editSeries).toHaveBeenCalledWith({ kind: 'remove', photo: 2 }, 10),
    );
    expect(await screen.findByText('IMG_3.jpg', { selector: '.viewer-name' })).toBeInTheDocument();
    const toast = await screen.findByText('Wyjęto IMG_2.jpg z serii');
    // The toast offers „Cofnij” next to the bar's button.
    expect(screen.getAllByRole('button', { name: 'Cofnij' })).toHaveLength(2);
    expect(toast).toBeInTheDocument();
  });

  it('shows the photo before after taking out the last photo of the series', async () => {
    await openOn(3);

    press('w');

    expect(await screen.findByText('IMG_2.jpg', { selector: '.viewer-name' })).toBeInTheDocument();
  });

  it('does nothing for P on the first photo, nor W or P on a single photo', async () => {
    await openOn(1);
    press('p');
    await openOnAgain(0);
    press('w');
    press('p');

    expect(editSeries).not.toHaveBeenCalled();
  });

  it('takes out one photo per press of W, not more while it is held', async () => {
    await openOn(1);

    press('w');
    expect(await screen.findByText('Wyjęto IMG_1.jpg z serii')).toBeInTheDocument();
    press('w', { repeat: true });
    press('w', { repeat: true });

    expect(editSeries).toHaveBeenCalledOnce();
  });

  it('keeps the viewer keys out while the menu is open', async () => {
    vi.mocked(rejectPhoto).mockResolvedValue({ name: 'IMG_2.jpg' });
    await openOn(2);
    await userEvent.click(editButton());

    for (const key of ['ArrowRight', 'ArrowLeft', 'Delete', 'a', 'p', 'w']) press(key);
    press('Escape');

    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
    expect(closeWindow).not.toHaveBeenCalled();
    expect(rejectPhoto).not.toHaveBeenCalled();
    expect(editSeries).not.toHaveBeenCalled();
    expect(shownName()).toBe('IMG_2.jpg');
    expect(editButton()).toHaveFocus();
    // Closed, Escape closes the viewer again.
    press('Escape');
    expect(closeWindow).toHaveBeenCalledOnce();
  });

  it('reports a failed edit', async () => {
    vi.mocked(editSeries).mockRejectedValue({ code: 'seriesChanged' });
    await openOn(2);

    press('w');

    expect(
      await screen.findByText('Nie można zmienić serii – zmieniła się w międzyczasie.'),
    ).toBeInTheDocument();
    expect(shownName()).toBe('IMG_2.jpg');
  });
});
