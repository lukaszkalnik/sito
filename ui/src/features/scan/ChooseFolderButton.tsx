import { useTranslation } from 'react-i18next';

import '../../controls/controls.css';
import { FolderIcon } from './FolderIcon';

/** „Wybierz inny folder” as an icon button, disabled while `busy`. */
export function ChooseFolderButton({ busy, onClick }: { busy: boolean; onClick: () => void }) {
  const { t } = useTranslation();
  const label = t('scan.chooseAnother');
  return (
    <button
      type="button"
      className="control-icon-button"
      aria-label={label}
      title={label}
      disabled={busy}
      onClick={onClick}
    >
      <FolderIcon />
    </button>
  );
}
