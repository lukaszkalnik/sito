import { useTranslation } from 'react-i18next';

import '../../controls/controls.css';
import { onFirstClick } from '../../clicks/firstClick';
import { RefreshIcon } from './RefreshIcon';

/** „Odśwież” as an icon button: checks the open folder for changes, disabled while `busy`. */
export function RefreshButton({ busy, onClick }: { busy: boolean; onClick: () => void }) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className="control-icon-button"
      aria-label={t('scan.refresh')}
      title={t('scan.refreshHint')}
      disabled={busy}
      onClick={onFirstClick(onClick)}
    >
      <RefreshIcon />
    </button>
  );
}
