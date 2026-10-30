import { useCallback, useEffect, useReducer, useRef, useState, type RefObject } from 'react';

import { useShortcuts } from '../../shortcuts/useShortcuts';
import type { ViewerPhoto } from './api';
import {
  FIT,
  layoutOf,
  zoomReducer,
  type Geometry,
  type Layout,
  type Point,
  type Size,
  type Zoom,
  type ZoomAction,
} from './zoom';

/** What the view needs to draw the zoomed photo and control the zoom. */
export interface ZoomView {
  layout: Layout;
  /** The zoom in screen pixels per photo pixel while zoomed, `null` at fit. */
  scale: number | null;
  /** Takes the size of a photo that has just loaded. */
  onPhotoSize: (size: Size) => void;
  fit: () => void;
}

/** The zoom of the shown photo. */
export interface PhotoZoom {
  /** The element the photo is drawn in; zooming measures it and listens to it. */
  viewRef: RefObject<HTMLDivElement | null>;
  view: ZoomView;
}

/** How much `+` and `-` zoom in or out. */
const KEY_FACTOR = 1.25;
/** How far Shift+arrows move the view, in view pixels. */
const PAN_STEP = 120;
/** How fast the wheel zooms: a factor of e per this many pixels of wheel movement. */
const WHEEL_PIXELS_PER_E = 400;
/** Pixels per line, for wheels that report lines (`WheelEvent.DOM_DELTA_LINE`). */
const LINE_PIXELS = 16;

const NO_SIZE: Size = { width: 0, height: 0 };

/**
 * Zoom and position of the photo in the viewer. They carry over to another
 * photo of the same series (relative to the photo, so the same spot stays in
 * view) and reset to fit on any other photo. Wheel and pinch zoom around the
 * pointer, a drag pans, a double-click switches between fit and 100 %, and the
 * zoom keys of `shortcuts/keymap.ts` apply.
 */
export function usePhotoZoom(photo: ViewerPhoto): PhotoZoom {
  const viewRef = useRef<HTMLDivElement>(null);
  const view = useViewSize(viewRef);
  const [photoSize, setPhotoSize] = useState<Size>(NO_SIZE);
  const geometry: Geometry = { photo: photoSize, view, pixelRatio: window.devicePixelRatio || 1 };
  const [zoom, dispatch] = useReducer(
    (state: Zoom, { action, at }: { action: ZoomAction; at: Geometry }) =>
      zoomReducer(state, action, at),
    FIT,
  );
  // Listeners are attached once, so they read the latest geometry from here.
  const latest = useRef(geometry);
  useEffect(() => {
    latest.current = geometry;
  });
  const act = useCallback((action: ZoomAction) => dispatch({ action, at: latest.current }), []);

  useResetOnPhotoChange(photo, act);
  usePointerZoom(viewRef, act);
  const layout = layoutOf(zoom, geometry);
  const zoomed = layout.kind === 'zoomed' && zoom.kind === 'zoomed';
  useZoomKeys(zoomed, act);
  const fit = useCallback(() => act({ kind: 'fit' }), [act]);

  return {
    viewRef,
    view: { layout, scale: zoomed ? zoom.scale : null, onPhotoSize: setPhotoSize, fit },
  };
}

/** The size of `ref`'s element in CSS pixels, kept up to date as it resizes. */
function useViewSize(ref: RefObject<HTMLElement | null>): Size {
  const [size, setSize] = useState<Size>(NO_SIZE);
  useEffect(() => {
    const element = ref.current;
    if (!element || typeof ResizeObserver === 'undefined') return;
    const observer = new ResizeObserver(([first]) => {
      if (!first) return;
      const { width, height } = first.contentRect;
      setSize({ width, height });
    });
    observer.observe(element);
    return () => observer.disconnect();
  }, [ref]);
  return size;
}

/**
 * Tells the zoom when another photo is shown, and whether it is in the
 * series of the previous one. That also holds after a rejection, which shows
 * the next shot of the same series.
 */
function useResetOnPhotoChange(photo: ViewerPhoto, act: (action: ZoomAction) => void): void {
  const previous = useRef(photo);
  useEffect(() => {
    const before = previous.current;
    if (before.entry.id === photo.entry.id) return;
    const sameSeries = before.series.some((member) => member.id === photo.entry.id);
    act({ kind: 'photoChanged', sameSeries });
    previous.current = photo;
  }, [photo, act]);
}

/** The keys of `shortcuts/keymap.ts`; zooming out and panning only while zoomed. */
function useZoomKeys(zoomed: boolean, act: (action: ZoomAction) => void): void {
  const whileZoomed = (action: ZoomAction) => (zoomed ? () => act(action) : null);
  useShortcuts({
    zoomIn: () => act({ kind: 'zoomBy', factor: KEY_FACTOR }),
    zoomOut: whileZoomed({ kind: 'zoomBy', factor: 1 / KEY_FACTOR }),
    zoomFit: whileZoomed({ kind: 'fit' }),
    zoomActual: () => act({ kind: 'actual' }),
    panLeft: whileZoomed({ kind: 'panBy', dx: -PAN_STEP, dy: 0 }),
    panRight: whileZoomed({ kind: 'panBy', dx: PAN_STEP, dy: 0 }),
    panUp: whileZoomed({ kind: 'panBy', dx: 0, dy: -PAN_STEP }),
    panDown: whileZoomed({ kind: 'panBy', dx: 0, dy: PAN_STEP }),
  });
}

/** Where `event` happened, in pixels from the top-left corner of `element`. */
function pointIn(element: HTMLElement, event: MouseEvent): Point {
  const rect = element.getBoundingClientRect();
  return { x: event.clientX - rect.left, y: event.clientY - rect.top };
}

/** Whether `event` started on a control (e.g. „Dopasuj”), which handles it itself. */
function onControl(event: Event): boolean {
  return event.target instanceof Element && event.target.closest('button') !== null;
}

/** The zoom factor of one wheel event: up or a pinch apart zooms in. */
function wheelFactor(event: WheelEvent): number {
  const pixels = event.deltaMode === WheelEvent.DOM_DELTA_LINE ? LINE_PIXELS : 1;
  return Math.exp((-event.deltaY * pixels) / WHEEL_PIXELS_PER_E);
}

/** Wheel and pinch zoom, drag to pan and double-click to toggle, on `ref`'s element. */
function usePointerZoom(
  ref: RefObject<HTMLElement | null>,
  act: (action: ZoomAction) => void,
): void {
  useEffect(() => {
    const element = ref.current;
    if (!element) return;
    const onWheel = (event: WheelEvent) => {
      // Not passive: a pinch arrives as Ctrl+wheel, which would zoom the whole page.
      event.preventDefault();
      act({ kind: 'zoomBy', factor: wheelFactor(event), at: pointIn(element, event) });
    };
    const onDoubleClick = (event: MouseEvent) => {
      if (!onControl(event)) act({ kind: 'toggle', at: pointIn(element, event) });
    };
    const stopDrag = startDrag(element, act);
    element.addEventListener('wheel', onWheel, { passive: false });
    element.addEventListener('dblclick', onDoubleClick);
    return () => {
      element.removeEventListener('wheel', onWheel);
      element.removeEventListener('dblclick', onDoubleClick);
      stopDrag();
    };
  }, [ref, act]);
}

/** Pans while the photo is dragged with the main button; returns a function that stops listening. */
function startDrag(element: HTMLElement, act: (action: ZoomAction) => void): () => void {
  let last: Point | null = null;
  const onDown = (event: PointerEvent) => {
    if (event.button !== 0 || onControl(event)) return;
    last = { x: event.clientX, y: event.clientY };
    element.setPointerCapture(event.pointerId);
  };
  const onMove = (event: PointerEvent) => {
    if (last === null) return;
    // Dragging the photo right shows what lies to its left.
    act({ kind: 'panBy', dx: last.x - event.clientX, dy: last.y - event.clientY });
    last = { x: event.clientX, y: event.clientY };
  };
  const onUp = () => {
    last = null;
  };
  element.addEventListener('pointerdown', onDown);
  element.addEventListener('pointermove', onMove);
  element.addEventListener('pointerup', onUp);
  element.addEventListener('pointercancel', onUp);
  return () => {
    element.removeEventListener('pointerdown', onDown);
    element.removeEventListener('pointermove', onMove);
    element.removeEventListener('pointerup', onUp);
    element.removeEventListener('pointercancel', onUp);
  };
}
