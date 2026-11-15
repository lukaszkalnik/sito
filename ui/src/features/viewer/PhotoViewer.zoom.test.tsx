import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { PhotoViewer } from './PhotoViewer';
import { pageOf, renderViewer, viewerProps } from './photoViewerFixtures';

vi.mock('./api', () => ({
  photoUrl: (scanId: number, id: number) => `photo://localhost/${id}?scan=${scanId}`,
  setWindowTitle: vi.fn(() => Promise.resolve()),
  checkPhotoFile: vi.fn(() => Promise.resolve(null)),
}));
vi.mock('../reject/api', async (importActual) => ({
  ...(await importActual<typeof import('../reject/api')>()),
  onTimelineChanged: vi.fn(() => Promise.resolve(() => {})),
}));
vi.mock('../timeline/thumbnails', () => ({
  thumbnailUrl: (scanId: number, id: number) => `thumb://localhost/${id}?scan=${scanId}`,
}));

/** A view of 1000×500 pixels, as jsdom does no layout. */
class FixedSizeObserver {
  private readonly callback: ResizeObserverCallback;
  constructor(callback: ResizeObserverCallback) {
    this.callback = callback;
  }
  observe() {
    const entry = { contentRect: { width: 1000, height: 500 } } as ResizeObserverEntry;
    this.callback([entry], this as unknown as ResizeObserver);
  }
  disconnect() {}
  unobserve() {}
}

/** Lets the loading photo finish as a 2000×1000 image. */
function loadPhoto() {
  const image = screen.getByTestId('viewer-photo-loading');
  Object.defineProperty(image, 'naturalWidth', { value: 2000 });
  Object.defineProperty(image, 'naturalHeight', { value: 1000 });
  fireEvent.load(image);
}

const zoomBadge = () => screen.queryByRole('group', { name: 'Powiększenie' });

describe('PhotoViewer zoom', () => {
  beforeEach(() => {
    vi.stubGlobal('ResizeObserver', FixedSizeObserver);
    vi.clearAllMocks();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('shows the zoom level only while zoomed', async () => {
    renderViewer(pageOf(2));
    loadPhoto();
    expect(zoomBadge()).toBeNull();

    await userEvent.keyboard('1');
    expect(zoomBadge()).toHaveTextContent('100%');
    await userEvent.keyboard('+');
    expect(zoomBadge()).toHaveTextContent('125%');

    await userEvent.keyboard('0');
    expect(zoomBadge()).toBeNull();
  });

  it('goes back to fit with „Dopasuj”', async () => {
    renderViewer(pageOf(2));
    loadPhoto();
    await userEvent.keyboard('1');

    await userEvent.click(screen.getByRole('button', { name: 'Dopasuj' }));

    expect(zoomBadge()).toBeNull();
  });

  it('pans with Shift+arrows instead of stepping', async () => {
    const { onShow } = renderViewer(pageOf(2));
    loadPhoto();
    await userEvent.keyboard('1');

    await userEvent.keyboard('{Shift>}{ArrowRight}{/Shift}');

    expect(onShow).not.toHaveBeenCalled();
    expect(screen.getByRole('img', { name: 'IMG_2.jpg' }).style.transform).toBe(
      'translate(-620px, -250px)',
    );
  });

  it('keeps the zoom within a series and resets it outside', async () => {
    const props = viewerProps(pageOf(2));
    const { rerender } = render(<PhotoViewer {...props} />);
    loadPhoto();
    await userEvent.keyboard('1');

    rerender(<PhotoViewer {...props} photo={pageOf(3)} />);
    expect(zoomBadge()).toHaveTextContent('100%');

    rerender(<PhotoViewer {...props} photo={pageOf(4)} />);
    expect(zoomBadge()).toBeNull();
  });
});
