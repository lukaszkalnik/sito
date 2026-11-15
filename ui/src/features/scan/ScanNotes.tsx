import { useTranslation } from 'react-i18next';

import type { Completing } from './scanReducer';
import './ScanNotes.css';

interface ScanNotesProps {
  /** Whether the folder is being checked for changes in the background. */
  refreshing: boolean;
  /** Progress of reading EXIF in the background, `null` when none is read. */
  completing: Completing | null;
  /** Whether some photos of the folder are gone from disk; offers „Odśwież”. */
  photosMissing: boolean;
  onRefresh: () => void;
}

/**
 * A quiet note at the bottom of the window about work running in the
 * background, or about photos gone from disk with a button to refresh.
 */
export function ScanNotes({ refreshing, completing, photosMissing, onRefresh }: ScanNotesProps) {
  const { t } = useTranslation();
  const notes = [
    refreshing ? t('scan.refreshing') : null,
    completing ? t('scan.completing', { ...completing }) : null,
  ].filter((note) => note !== null);
  return (
    // Always rendered, so screen readers announce new notes.
    <div className="scan-notes-area" aria-live="polite">
      {notes.length > 0 && <div className="scan-notes">{notes.join(' ')}</div>}
      {photosMissing && (
        <div className="scan-notes scan-notes--action">
          <span>{t('scan.photosMissing')}</span>
          <button type="button" onClick={onRefresh}>
            {t('scan.refresh')}
          </button>
        </div>
      )}
    </div>
  );
}
