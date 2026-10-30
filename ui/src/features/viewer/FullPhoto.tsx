import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { useMountedFor } from '../timeline/useMountedFor';

/**
 * The photo at full size. While the next photo loads, the previous one
 * stays for a moment, so stepping through photos never flashes an empty
 * window; a photo that takes longer shows a spinner instead. If it cannot
 * be loaded, only the error is shown.
 */
export function FullPhoto({ src, alt }: { src: string; alt: string }) {
  const { t } = useTranslation();
  const [loaded, setLoaded] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  const loading = loaded !== src && failed !== src;

  return (
    <div className="viewer-photo-area">
      {loaded === src && <img className="viewer-photo" src={src} alt={alt} />}
      {loading && <WhileLoading key={src} previous={loaded} alt={alt} />}
      {loading && (
        <img
          className="viewer-photo viewer-photo--loading"
          src={src}
          alt=""
          data-testid="viewer-photo-loading"
          onLoad={() => setLoaded(src)}
          onError={() => setFailed(src)}
        />
      )}
      {failed === src && (
        <p role="alert" className="viewer-error">
          {t('viewer.loadError')}
        </p>
      )}
    </div>
  );
}

/** How long the previous photo stays while the next one loads, before the spinner replaces it. */
const SPINNER_DELAY_MS = 300;

/**
 * What the viewer shows while a photo loads: the previously shown photo at
 * first, then a spinner on its own (e.g. while a TIFF is converted), so the
 * old photo never looks like the one that is loading.
 */
function WhileLoading({ previous, alt }: { previous: string | null; alt: string }) {
  const slow = useMountedFor(SPINNER_DELAY_MS);
  if (!slow) {
    return previous === null ? null : <img className="viewer-photo" src={previous} alt={alt} />;
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
