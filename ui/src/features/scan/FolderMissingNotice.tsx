import { useTranslation } from 'react-i18next';

import '../../controls/controls.css';
import './FolderMissingNotice.css';

/** Below the bar, when the open folder is gone from disk: says so and offers to choose another. */
export function FolderMissingNotice({ onChooseAnother }: { onChooseAnother: () => void }) {
  const { t } = useTranslation();
  return (
    <div className="folder-missing" role="alert">
      <p>{t('scan.folderMissing')}</p>
      <button type="button" className="control-button" onClick={onChooseAnother}>
        {t('scan.chooseAnother')}
      </button>
    </div>
  );
}
