import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { canUndo, onTimelineChanged, rejectPhoto } from '../reject/api';
import type { TimelineEntry } from '../timeline/api';
import { getTimeline } from '../timeline/api';
import {
  getViewerPhoto,
  onViewerContext,
  onViewerShow,
  currentViewerRequest,
  type ViewerPhoto,
} from './api';
import { ViewerApp } from './ViewerApp';

vi.mock('../timeline/api', async (importActual) => ({
  ...(await importActual<typeof import('../timeline/api')>()),
  getTimeline: vi.fn(),
}));
vi.mock('../reject/api', async (importActual) => ({
  ...(await importActual<typeof import('../reject/api')>()),
  canUndo: vi.fn(),
  onTimelineChanged: vi.fn(),
  rejectPhoto: vi.fn(),
}));
vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  currentViewerRequest: vi.fn(),
  getViewerPhoto: vi.fn(),
  onViewerShow: vi.fn(),
  onViewerContext: vi.fn(),
  photoUrl: (scanId: number, id: number) => `photo://localhost/${id}?scan=${scanId}`,
  setWindowTitle: vi.fn(() => Promise.resolve()),
}));

function entry(id: number): TimelineEntry {
  return {
    id,
    name: `IMG_${id}.jpg`,
    relativeDir: '',
    relativePath: `IMG_${id}.jpg`,
    takenAt: '2024-06-01T12:00:00',
    timeSource: 'exif',
    format: 'jpeg',
  };
}

/** Photos 3 and 4, each continuing with the other after a rejection. */
function page(id: number, afterReject: number | null): ViewerPhoto {
  return { entry: entry(id), previous: null, next: null, series: [], afterReject };
}

const spinner = () => screen.queryByRole('status', { name: 'Wczytywanie zdjęcia…' });

describe('ViewerApp', () => {
  beforeEach(() => {
    localStorage.clear();
    vi.resetAllMocks();
    const noUnlisten = () => Promise.resolve(() => {});
    vi.mocked(onViewerShow).mockImplementation(noUnlisten);
    vi.mocked(onViewerContext).mockImplementation(noUnlisten);
    vi.mocked(onTimelineChanged).mockImplementation(noUnlisten);
    vi.mocked(canUndo).mockResolvedValue(false);
    vi.mocked(currentViewerRequest).mockResolvedValue({ id: 3, scanId: 1, mode: 'all', gap: 10 });
  });

  it('shows a spinner while it waits for its photo', () => {
    vi.mocked(currentViewerRequest).mockReturnValue(new Promise(() => {}));

    render(<ViewerApp />);

    expect(spinner()).toBeInTheDocument();
  });

  it('shows a spinner while the photo is looked up', async () => {
    vi.mocked(getViewerPhoto).mockReturnValue(new Promise(() => {}));

    render(<ViewerApp />);

    await vi.waitFor(() => expect(getViewerPhoto).toHaveBeenCalled());
    expect(spinner()).toBeInTheDocument();
  });

  it('shows its photo without loading the whole timeline', async () => {
    vi.mocked(getViewerPhoto).mockResolvedValue(page(3, null));

    render(<ViewerApp />);

    expect(await screen.findByText('IMG_3.jpg')).toBeInTheDocument();
    expect(getViewerPhoto).toHaveBeenCalledWith({ id: 3, scanId: 1, mode: 'all', gap: 10 });
    expect(getTimeline).not.toHaveBeenCalled();
  });

  it('continues with the photo the backend picked after a rejection', async () => {
    vi.mocked(getViewerPhoto).mockImplementation((request) =>
      Promise.resolve(page(request.id, request.id === 3 ? 4 : null)),
    );
    vi.mocked(rejectPhoto).mockResolvedValue({ name: 'IMG_3.jpg' });
    render(<ViewerApp />);
    await screen.findByText('IMG_3.jpg', { selector: '.viewer-name' });

    await userEvent.click(screen.getByRole('button', { name: 'Odrzuć' }));

    expect(rejectPhoto).toHaveBeenCalledWith(3, 'folder');
    expect(await screen.findByText('IMG_4.jpg', { selector: '.viewer-name' })).toBeInTheDocument();
  });

  it('says so once no photo is left', async () => {
    vi.mocked(getViewerPhoto).mockResolvedValue(page(3, null));
    vi.mocked(rejectPhoto).mockResolvedValue({ name: 'IMG_3.jpg' });
    render(<ViewerApp />);
    await screen.findByText('IMG_3.jpg', { selector: '.viewer-name' });

    await userEvent.click(screen.getByRole('button', { name: 'Odrzuć' }));

    expect(await screen.findByText('Brak zdjęć do pokazania.')).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Odrzuć' })).toBeNull();
  });

  it('rejects into the trash once it is chosen, and says so', async () => {
    localStorage.setItem('sito.rejectDestination', 'trash');
    vi.mocked(getViewerPhoto).mockImplementation((request) =>
      Promise.resolve(page(request.id, request.id === 3 ? 4 : null)),
    );
    vi.mocked(rejectPhoto).mockResolvedValue({ name: 'IMG_3.jpg' });
    render(<ViewerApp />);
    await screen.findByText('IMG_3.jpg', { selector: '.viewer-name' });

    const button = screen.getByRole('button', { name: 'Odrzuć' });
    expect(button).toHaveAttribute('title', 'Przenieś zdjęcie do kosza (Delete)');
    await userEvent.click(button);

    expect(rejectPhoto).toHaveBeenCalledWith(3, 'trash');
    expect(await screen.findByText('IMG_3.jpg → Kosz')).toBeInTheDocument();
  });

  it('explains a drive without a trash', async () => {
    localStorage.setItem('sito.rejectDestination', 'trash');
    vi.mocked(getViewerPhoto).mockResolvedValue(page(3, null));
    vi.mocked(rejectPhoto).mockRejectedValue({ code: 'noTrash' });
    render(<ViewerApp />);
    await screen.findByText('IMG_3.jpg', { selector: '.viewer-name' });

    await userEvent.click(screen.getByRole('button', { name: 'Odrzuć' }));

    expect(await screen.findByRole('alert')).toHaveTextContent(
      'Ten dysk nie ma kosza – wybierz folder „Odrzucone”.',
    );
  });
});
