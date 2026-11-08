import { useTranslation } from 'react-i18next';

import { onFirstClick } from '../../clicks/firstClick';

/** „Cofnij”: undoes the newest rejection, disabled while there is none. */
export function UndoButton({
  canUndo,
  onUndo,
  className,
}: {
  canUndo: boolean;
  onUndo: () => void;
  className?: string;
}) {
  const { t } = useTranslation();
  return (
    <button
      type="button"
      className={className}
      title={t('undo.hint')}
      disabled={!canUndo}
      onClick={onFirstClick(onUndo)}
    >
      {t('undo.button')}
    </button>
  );
}
