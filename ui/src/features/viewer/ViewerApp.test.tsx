import { render, screen } from '@testing-library/react';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { getSeries, getTimeline } from '../timeline/api';
import { takeViewerRequest } from './api';
import { ViewerApp } from './ViewerApp';

vi.mock('../timeline/api', async (importActual) => ({
  ...(await importActual<typeof import('../timeline/api')>()),
  getTimeline: vi.fn(),
  getSeries: vi.fn(),
}));
vi.mock('../reject/api', async (importActual) => ({
  ...(await importActual<typeof import('../reject/api')>()),
  canUndo: vi.fn(() => Promise.resolve(false)),
  onTimelineChanged: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock('./api', async (importActual) => ({
  ...(await importActual<typeof import('./api')>()),
  takeViewerRequest: vi.fn(),
  onViewerShow: vi.fn(() => Promise.resolve(() => {})),
  onViewerContext: vi.fn(() => Promise.resolve(() => {})),
}));

describe('ViewerApp', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  it('shows a spinner while it waits for its photo', () => {
    vi.mocked(takeViewerRequest).mockReturnValue(new Promise(() => {}));

    render(<ViewerApp />);

    expect(screen.getByRole('status', { name: 'Wczytywanie zdjęcia…' })).toBeInTheDocument();
  });

  it('shows a spinner while the timeline around the photo loads', async () => {
    vi.mocked(takeViewerRequest).mockResolvedValue({ id: 3, scanId: 1, mode: 'all', gap: 10 });
    vi.mocked(getTimeline).mockReturnValue(new Promise(() => {}));
    vi.mocked(getSeries).mockReturnValue(new Promise(() => {}));

    render(<ViewerApp />);

    await vi.waitFor(() => expect(getTimeline).toHaveBeenCalled());

    expect(screen.getByRole('status', { name: 'Wczytywanie zdjęcia…' })).toBeInTheDocument();
  });
});
