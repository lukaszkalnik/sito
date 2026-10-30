import { describe, expect, it } from 'vitest';

import {
  FIT,
  layoutOf,
  MAX_SCALE,
  zoomReducer,
  type Geometry,
  type Point,
  type Zoom,
  type ZoomAction,
} from './zoom';

/** A 2000×1000 photo in a 1000×500 view: fitted at half size. */
const GEOMETRY: Geometry = {
  photo: { width: 2000, height: 1000 },
  view: { width: 1000, height: 500 },
  pixelRatio: 1,
};

function run(actions: readonly ZoomAction[], geometry: Geometry = GEOMETRY): Zoom {
  return actions.reduce((zoom, action) => zoomReducer(zoom, action, geometry), FIT);
}

function zoomed(zoom: Zoom): Extract<Zoom, { kind: 'zoomed' }> {
  if (zoom.kind !== 'zoomed') throw new Error('expected a zoomed photo');
  return zoom;
}

/** The photo spot (as a fraction) drawn at view point `at`. */
function spotAt(zoom: Zoom, at: Point, geometry: Geometry = GEOMETRY): Point {
  const layout = layoutOf(zoom, geometry);
  if (layout.kind === 'fit') {
    const scale = Math.min(
      geometry.view.width / geometry.photo.width,
      geometry.view.height / geometry.photo.height,
    );
    const width = geometry.photo.width * scale;
    const height = geometry.photo.height * scale;
    return {
      x: (at.x - (geometry.view.width - width) / 2) / width,
      y: (at.y - (geometry.view.height - height) / 2) / height,
    };
  }
  return { x: (at.x - layout.left) / layout.width, y: (at.y - layout.top) / layout.height };
}

describe('zoomReducer', () => {
  it('keeps the spot under the pointer in place while zooming', () => {
    const at = { x: 250, y: 125 };
    const before = spotAt(FIT, at);

    const zoom = run([{ kind: 'zoomBy', factor: 4, at }]);

    expect(zoomed(zoom).scale).toBe(2);
    expect(spotAt(zoom, at).x).toBeCloseTo(before.x);
    expect(spotAt(zoom, at).y).toBeCloseTo(before.y);
  });

  it('zooms around the middle without a pointer', () => {
    const zoom = run([{ kind: 'zoomBy', factor: 2 }]);

    expect(zoom).toEqual({ kind: 'zoomed', scale: 1, center: { x: 0.5, y: 0.5 } });
  });

  it('snaps back to fit when zooming out past it', () => {
    expect(run([{ kind: 'actual' }, { kind: 'zoomBy', factor: 0.4 }])).toEqual(FIT);
    expect(run([{ kind: 'zoomBy', factor: 0.8 }])).toEqual(FIT);
  });

  it('stops at 800 %', () => {
    const zoom = run([{ kind: 'actual' }, { kind: 'zoomBy', factor: 100 }]);

    expect(zoomed(zoom).scale).toBe(MAX_SCALE);
  });

  it('pans only as far as the edges of the photo', () => {
    const zoom = run([{ kind: 'actual' }, { kind: 'panBy', dx: 10_000, dy: -10_000 }]);

    // At 100 % the 1000×500 view shows a quarter of the width and half the height.
    expect(zoomed(zoom).center).toEqual({ x: 0.75, y: 0.25 });
    expect(layoutOf(zoom, GEOMETRY)).toEqual({
      kind: 'zoomed',
      width: 2000,
      height: 1000,
      left: -1000,
      top: 0,
    });
  });

  it('does not pan a fitted photo', () => {
    expect(run([{ kind: 'panBy', dx: 100, dy: 100 }])).toEqual(FIT);
  });

  it('keeps an axis smaller than the view centred', () => {
    const wide: Geometry = { ...GEOMETRY, photo: { width: 2000, height: 200 } };

    const zoom = run(
      [
        { kind: 'actual', at: { x: 0, y: 0 } },
        { kind: 'panBy', dx: 0, dy: 300 },
      ],
      wide,
    );

    expect(zoomed(zoom).center.y).toBe(0.5);
  });

  it('measures 100 % in screen pixels, whatever the display scaling', () => {
    const scaled: Geometry = { ...GEOMETRY, pixelRatio: 1.25 };

    const layout = layoutOf(run([{ kind: 'actual' }], scaled), scaled);

    expect(layout).toMatchObject({ kind: 'zoomed', width: 1600, height: 800 });
  });

  it('switches between fit and 100 % at the clicked spot', () => {
    const at = { x: 750, y: 250 };
    const zoom = run([{ kind: 'toggle', at }]);

    expect(zoomed(zoom).scale).toBe(1);
    expect(spotAt(zoom, at).x).toBeCloseTo(spotAt(FIT, at).x);
    expect(zoomReducer(zoom, { kind: 'toggle', at }, GEOMETRY)).toEqual(FIT);
  });

  it('keeps the zoom within a series and resets it outside', () => {
    const zoom = run([{ kind: 'actual', at: { x: 900, y: 400 } }]);

    expect(zoomReducer(zoom, { kind: 'photoChanged', sameSeries: true }, GEOMETRY)).toBe(zoom);
    expect(zoomReducer(zoom, { kind: 'photoChanged', sameSeries: false }, GEOMETRY)).toEqual(FIT);
    expect(zoomReducer(zoom, { kind: 'fit' }, GEOMETRY)).toEqual(FIT);
  });

  it('waits for the photo to be measured before zooming', () => {
    const unmeasured: Geometry = { ...GEOMETRY, photo: { width: 0, height: 0 } };

    expect(run([{ kind: 'zoomBy', factor: 2 }], unmeasured)).toEqual(FIT);
  });
});

describe('layoutOf', () => {
  it('shows the same relative spot on a photo of another size', () => {
    const zoom = run([{ kind: 'actual' }, { kind: 'panBy', dx: 200, dy: 100 }]);
    const bigger: Geometry = { ...GEOMETRY, photo: { width: 4000, height: 2000 } };
    const middle = { x: 500, y: 250 };

    expect(spotAt(zoom, middle, bigger).x).toBeCloseTo(zoomed(zoom).center.x);
    expect(spotAt(zoom, middle, bigger).y).toBeCloseTo(zoomed(zoom).center.y);
  });

  it('draws a zoom at or below fit fitted, e.g. after the window grew', () => {
    const zoom = run([{ kind: 'actual' }]);
    const large: Geometry = { ...GEOMETRY, view: { width: 4000, height: 2000 } };

    expect(layoutOf(zoom, large)).toEqual({ kind: 'fit' });
  });
});
