import { useTranslation } from 'react-i18next';

import '../../controls/controls.css';

/** Outline icon of an open folder, in the current text colour. */
function FolderIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="18"
      height="18"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M6 14l1.5-3A2 2 0 0 1 9.3 10H21a1 1 0 0 1 1 1.2l-1.5 6A2 2 0 0 1 18.5 19H4a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h4l2 2h7a2 2 0 0 1 2 2v2" />
    </svg>
  );
}

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
