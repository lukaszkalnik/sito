import type { ViewerPhoto, ViewerRequest } from './api';

/** Most pages kept for one view; stepping through more starts over. */
const MAX_PAGES = 200;

/** Loads the page of one photo. */
export type LoadPage = (request: ViewerRequest) => Promise<ViewerPhoto | null>;

/**
 * Viewer pages of one view (scan, mode, gap and timeline revision), each
 * loaded once, so stepping to a prefetched photo needs no new request. A
 * request for another view drops the pages of the previous one; a failed
 * load is forgotten, so the next request tries again.
 */
export class ViewerPageCache {
  private view = '';
  private readonly pages = new Map<number, Promise<ViewerPhoto | null>>();
  private readonly load: LoadPage;

  constructor(load: LoadPage) {
    this.load = load;
  }

  /** The page of `request` at timeline `revision`. */
  get(request: ViewerRequest, revision: number): Promise<ViewerPhoto | null> {
    const view = `${request.scanId}/${request.mode}/${request.gap}/${revision}`;
    if (view !== this.view || this.pages.size >= MAX_PAGES) {
      this.pages.clear();
      this.view = view;
    }
    const cached = this.pages.get(request.id);
    if (cached) return cached;
    const page = this.load(request);
    this.pages.set(request.id, page);
    page.catch(() => {
      if (this.pages.get(request.id) === page) this.pages.delete(request.id);
    });
    return page;
  }
}
