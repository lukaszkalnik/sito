/** A width and height in pixels. */
export interface Size {
  readonly width: number;
  readonly height: number;
}

/** A point: in view pixels from the view's top-left corner, or as a fraction of the photo. */
export interface Point {
  readonly x: number;
  readonly y: number;
}

/**
 * What zooming depends on: the photo's own size in pixels, the size of the
 * view in CSS pixels, and how many screen pixels one CSS pixel has
 * (`devicePixelRatio`).
 */
export interface Geometry {
  readonly photo: Size;
  readonly view: Size;
  readonly pixelRatio: number;
}

/**
 * How the photo is shown: fitted into the view, or zoomed. `scale` is in
 * screen pixels per photo pixel, so 1 shows the photo's real pixels whatever
 * the display scaling; `center` is the photo spot in the middle of the view,
 * as a fraction of the photo's width and height (0..1).
 */
export type Zoom =
  | { readonly kind: 'fit' }
  | { readonly kind: 'zoomed'; readonly scale: number; readonly center: Point };

/** What can be done to the zoom. `at` is a point in view pixels; without it, the view's middle. */
export type ZoomAction =
  | { readonly kind: 'zoomBy'; readonly factor: number; readonly at?: Point }
  | { readonly kind: 'fit' }
  | { readonly kind: 'actual'; readonly at?: Point }
  | { readonly kind: 'toggle'; readonly at: Point }
  | { readonly kind: 'panBy'; readonly dx: number; readonly dy: number }
  | { readonly kind: 'photoChanged'; readonly sameSeries: boolean };

/** Where the photo is drawn: as fitted, or with its size and top-left corner in view pixels. */
export type Layout =
  | { readonly kind: 'fit' }
  | {
      readonly kind: 'zoomed';
      readonly width: number;
      readonly height: number;
      readonly left: number;
      readonly top: number;
    };

/** The largest zoom: 800 % of the photo's own pixels. */
export const MAX_SCALE = 8;

export const FIT: Zoom = { kind: 'fit' };

const MIDDLE: Point = { x: 0.5, y: 0.5 };

/** Whether the photo and the view have been measured. */
function measured({ photo, view }: Geometry): boolean {
  return photo.width > 0 && photo.height > 0 && view.width > 0 && view.height > 0;
}

/** The scale of the fitted photo, in screen pixels per photo pixel. */
export function fitScale({ photo, view, pixelRatio }: Geometry): number {
  return Math.min(view.width / photo.width, view.height / photo.height) * pixelRatio;
}

/** Keeps a centre on one axis inside the photo; an axis smaller than the view stays centred. */
function clampAxis(center: number, shown: number, view: number): number {
  if (shown <= view) return 0.5;
  const half = view / 2 / shown;
  return Math.min(Math.max(center, half), 1 - half);
}

/** `center` moved so that the photo at `scale` never leaves an empty edge it could fill. */
export function clampCenter(center: Point, scale: number, geometry: Geometry): Point {
  const css = scale / geometry.pixelRatio;
  return {
    x: clampAxis(center.x, geometry.photo.width * css, geometry.view.width),
    y: clampAxis(center.y, geometry.photo.height * css, geometry.view.height),
  };
}

/** The scale and centre that `zoom` shows in `geometry`, fit included. */
function current(zoom: Zoom, geometry: Geometry): { scale: number; center: Point } {
  if (zoom.kind === 'fit' || zoom.scale <= fitScale(geometry)) {
    return { scale: fitScale(geometry), center: MIDDLE };
  }
  return { scale: zoom.scale, center: clampCenter(zoom.center, zoom.scale, geometry) };
}

/** The view's middle, in view pixels. */
function middleOf({ view }: Geometry): Point {
  return { x: view.width / 2, y: view.height / 2 };
}

/**
 * Zooms to `scale`, keeping the photo spot under `at` in place. A scale at
 * or below fit gives fit; above 800 % it stops there.
 */
function zoomTo(zoom: Zoom, scale: number, at: Point, geometry: Geometry): Zoom {
  const fit = fitScale(geometry);
  const target = Math.min(scale, Math.max(MAX_SCALE, fit));
  if (target <= fit) return FIT;
  const before = current(zoom, geometry);
  const { photo, pixelRatio } = geometry;
  const middle = middleOf(geometry);
  // The photo spot under `at`, as a fraction, before and after must match.
  const offset = (scale: number) => ({
    x: (at.x - middle.x) / ((photo.width * scale) / pixelRatio),
    y: (at.y - middle.y) / ((photo.height * scale) / pixelRatio),
  });
  const from = offset(before.scale);
  const to = offset(target);
  const spot = { x: before.center.x + from.x, y: before.center.y + from.y };
  const center = { x: spot.x - to.x, y: spot.y - to.y };
  return { kind: 'zoomed', scale: target, center: clampCenter(center, target, geometry) };
}

/** Moves the view by `dx`, `dy` view pixels over a zoomed photo; a fitted photo stays. */
function panBy(zoom: Zoom, dx: number, dy: number, geometry: Geometry): Zoom {
  if (zoom.kind === 'fit') return zoom;
  const { scale, center } = current(zoom, geometry);
  const css = scale / geometry.pixelRatio;
  const moved = {
    x: center.x + dx / (geometry.photo.width * css),
    y: center.y + dy / (geometry.photo.height * css),
  };
  return { kind: 'zoomed', scale, center: clampCenter(moved, scale, geometry) };
}

/** The zoom after `action`. Until the photo and the view are measured, only resets apply. */
export function zoomReducer(zoom: Zoom, action: ZoomAction, geometry: Geometry): Zoom {
  switch (action.kind) {
    case 'fit':
      return FIT;
    case 'photoChanged':
      return action.sameSeries ? zoom : FIT;
    case 'zoomBy':
    case 'actual':
    case 'toggle':
    case 'panBy':
      return measured(geometry) ? zoomMeasured(zoom, action, geometry) : zoom;
    default:
      return assertNever(action);
  }
}

/** The zoom after an action that needs the photo and the view measured. */
function zoomMeasured(
  zoom: Zoom,
  action: Extract<ZoomAction, { kind: 'zoomBy' | 'actual' | 'toggle' | 'panBy' }>,
  geometry: Geometry,
): Zoom {
  switch (action.kind) {
    case 'zoomBy': {
      const at = action.at ?? middleOf(geometry);
      return zoomTo(zoom, current(zoom, geometry).scale * action.factor, at, geometry);
    }
    case 'actual':
      return zoomTo(zoom, 1, action.at ?? middleOf(geometry), geometry);
    case 'toggle':
      return current(zoom, geometry).scale > fitScale(geometry)
        ? FIT
        : zoomTo(zoom, 1, action.at, geometry);
    case 'panBy':
      return panBy(zoom, action.dx, action.dy, geometry);
    default:
      return assertNever(action);
  }
}

/** Where to draw the photo for `zoom`; a zoom at or below fit (e.g. after a resize) is drawn fitted. */
export function layoutOf(zoom: Zoom, geometry: Geometry): Layout {
  if (zoom.kind === 'fit' || !measured(geometry)) return { kind: 'fit' };
  const { scale, center } = current(zoom, geometry);
  if (scale <= fitScale(geometry)) return { kind: 'fit' };
  const css = scale / geometry.pixelRatio;
  const width = geometry.photo.width * css;
  const height = geometry.photo.height * css;
  const middle = middleOf(geometry);
  return {
    kind: 'zoomed',
    width,
    height,
    left: middle.x - center.x * width,
    top: middle.y - center.y * height,
  };
}

function assertNever(value: never): never {
  throw new Error(`Unexpected zoom action: ${JSON.stringify(value)}`);
}
