import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { checkPhotoFile, type Vanished } from './api';

/** Why a photo failed to load, once the backend said. */
type Failure = { kind: 'checking' } | { kind: 'known'; vanished: Vanished | null };

/** Asks the backend once why photo `id` failed to load. */
function useFailure(id: number): Failure {
  const [failure, setFailure] = useState<{ id: number; failure: Failure } | null>(null);
  useEffect(() => {
    let active = true;
    checkPhotoFile(id).then(
      (vanished) => active && setFailure({ id, failure: { kind: 'known', vanished } }),
      () => active && setFailure({ id, failure: { kind: 'known', vanished: null } }),
    );
    return () => {
      active = false;
    };
  }, [id]);
  return failure?.id === id ? failure.failure : { kind: 'checking' };
}

/**
 * The message for photo `id` that failed to load: that its file or the
 * whole folder is gone from disk (with „Odśwież folder” for a file), or the
 * generic load error.
 */
export function PhotoError({ id, onRefresh }: { id: number; onRefresh: () => void }) {
  const { t } = useTranslation();
  const failure = useFailure(id);
  if (failure.kind === 'checking') return null;
  switch (failure.vanished) {
    case 'photo':
      return (
        <div role="alert" className="viewer-error">
          <p>{t('viewer.photoMissing')}</p>
          <button type="button" className="control-button" onClick={onRefresh}>
            {t('viewer.refreshFolder')}
          </button>
        </div>
      );
    case 'folder':
      return (
        <p role="alert" className="viewer-error">
          {t('viewer.folderMissing')}
        </p>
      );
    case null:
      return (
        <p role="alert" className="viewer-error">
          {t('viewer.loadError')}
        </p>
      );
  }
}
