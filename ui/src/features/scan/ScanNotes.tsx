import { useTranslation } from 'react-i18next';

import type { Completing } from './scanReducer';
import './ScanNotes.css';

interface ScanNotesProps {
  /** Whether the folder is being checked for changes in the background. */
  refreshing: boolean;
  /** Progress of reading EXIF in the background, `null` when none is read. */
  completing: Completing | null;
}

/** A quiet note at the bottom of the window about work running in the background. */
export function ScanNotes({ refreshing, completing }: ScanNotesProps) {
  const { t } = useTranslation();
  const notes = [
    refreshing ? t('scan.refreshing') : null,
    completing ? t('scan.completing', { ...completing }) : null,
  ].filter((note) => note !== null);
  return (
    // Always rendered, so screen readers announce new notes.
    <div className="scan-notes-area" aria-live="polite">
      {notes.length > 0 && <div className="scan-notes">{notes.join(' ')}</div>}
    </div>
  );
}
