import { useState, type CSSProperties, type RefObject } from 'react';
import { useTranslation } from 'react-i18next';

import { useMountedFor } from '../timeline/useMountedFor';
import type { ZoomView } from './usePhotoZoom';
import type { Layout } from './zoom';
import { ZoomBadge } from './ZoomBadge';

/**
 * The photo at full size. While the next photo loads, the previous one
 * stays for a moment, so stepping through photos never flashes an empty
 * window; a photo that takes longer shows a spinner instead. If it cannot
 * be loaded, only the error is shown. Both photos are drawn with `zoom`.
 */
export function FullPhoto({
  src,
  alt,
  zoom,
  viewRef,
}: {
  src: string;
  alt: string;
  zoom: ZoomView;
  viewRef: RefObject<HTMLDivElement | null>;
}) {
  const { t } = useTranslation();
  const [loaded, setLoaded] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const loading = loaded !== src && failed !== src;
  const zoomed = zoom.layout.kind === 'zoomed' ? ' viewer-photo-area--zoomed' : '';

  return (
    <div ref={viewRef} className={`viewer-photo-area${zoomed}`}>
      {loaded === src && <ShownPhoto src={src} alt={alt} layout={zoom.layout} />}
      {loading && <WhileLoading key={src} previous={loaded} alt={alt} layout={zoom.layout} />}
      {loading && (
        <img
          className="viewer-photo viewer-photo--loading"
          src={src}
          alt=""
          data-testid="viewer-photo-loading"
          onLoad={(event) => {
            const { naturalWidth: width, naturalHeight: height } = event.currentTarget;
            zoom.onPhotoSize({ width, height });
            setLoaded(src);
          }}
          onError={() => setFailed(src)}
        />
      )}
      {failed === src && (
        <p role="alert" className="viewer-error">
          {t('viewer.loadError')}
        </p>
      )}
      {zoom.scale !== null && <ZoomBadge scale={zoom.scale} onFit={zoom.fit} />}
    </div>
  );
}

/** The size and position of a zoomed photo; a fitted one keeps its CSS. */
function styleOf(layout: Layout): CSSProperties | undefined {
  if (layout.kind === 'fit') return undefined;
  return {
    width: layout.width,
    height: layout.height,
    transform: `translate(${layout.left}px, ${layout.top}px)`,
  };
}

/** A loaded photo, drawn as `layout` says. */
function ShownPhoto({ src, alt, layout }: { src: string; alt: string; layout: Layout }) {
  const zoomed = layout.kind === 'zoomed' ? ' viewer-photo--zoomed' : '';
  return (
    <img
      className={`viewer-photo${zoomed}`}
      src={src}
      alt={alt}
      style={styleOf(layout)}
      // The browser's own image drag would fight the pan.
      draggable={false}
    />
  );
}

/** How long the previous photo stays while the next one loads, before the spinner replaces it. */
const SPINNER_DELAY_MS = 300;

/**
 * What the viewer shows while a photo loads: the previously shown photo at
 * first, then a spinner on its own (e.g. while a TIFF is converted), so the
 * old photo never looks like the one that is loading.
 */
function WhileLoading({
  previous,
  alt,
  layout,
}: {
  previous: string | null;
  alt: string;
  layout: Layout;
}) {
  const slow = useMountedFor(SPINNER_DELAY_MS);
  if (!slow) {
    return previous === null ? null : <ShownPhoto src={previous} alt={alt} layout={layout} />;
  }
  return <LoadingSpinner />;
}

/** A spinner in the middle of the viewer, announced as „Wczytywanie zdjęcia…”. */
export function LoadingSpinner() {
  const { t } = useTranslation();
  return (
    <div className="viewer-loading" role="status" aria-label={t('viewer.loading')}>
      <div className="viewer-spinner" />
    </div>
  );
}
