import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';
import type { UndoState } from './api';

/**
 * „Cofnij”: undoes the newest rejection, disabled while there is none. With
 * only unrestorable trash moves left it looks disabled but stays clickable,
 * so a click can explain why.
 */
export function UndoButton({
  state,
  onUndo,
  className,
}: {
  state: UndoState;
  onUndo: () => void;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className={className}
      title={t('undo.hint')}
      disabled={state === 'none'}
      aria-disabled={state === 'trashOnly' ? true : undefined}
      onClick={onFirstClick(onUndo)}
    >
      {t('undo.button')}
    </button>
  );
}
