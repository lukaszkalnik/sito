import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';
import { CheckIcon } from './CheckIcon';
import './AcceptButton.css';

/** „Zaakceptuj”, pressed on an accepted photo, where it takes the mark off. */
export function AcceptButton({ accepted, onToggle }: { accepted: boolean; onToggle: () => void }) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className="accept-button"
      aria-pressed={accepted}
      title={t('accept.hint')}
      onClick={onFirstClick(onToggle)}
    >
      <CheckIcon size={20} />
      <span className="bar-button-label">
        {t(accepted ? 'accept.undoButton' : 'accept.button')}
      </span>
    </button>
  );
}
