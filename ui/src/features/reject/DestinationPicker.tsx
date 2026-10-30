import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { RejectDestination } from './destination';
import { DestinationIcon } from './DestinationIcon';
import { TrashConfirmDialog } from './TrashConfirmDialog';
import { useRejectDestination } from './useRejectDestination';
import './DestinationPicker.css';

/**
 * „Odrzucone zdjęcia do: [folderu „Odrzucone” | kosza]”. Switching to the
 * trash asks first; while the trash is chosen the control stands out in red.
 */
export function DestinationPicker({
  destination,
  onChange,
}: {
  destination: RejectDestination;
  onChange: (destination: RejectDestination) => void;
}) {
  const { t } = useTranslation();
  const [confirming, setConfirming] = useState(false);

  const choose = (next: RejectDestination) => {
    if (next === 'trash' && destination !== 'trash') setConfirming(true);
    else onChange(next);
  };
  const confirm = () => {
    setConfirming(false);
    onChange('trash');
  };

  return (
    <>
      <label
        className={`reject-destination${destination === 'trash' ? ' reject-destination--trash' : ''}`}
      >
        <DestinationIcon destination={destination} />
        <span>{t('destination.label')}</span>
        <select
          value={destination}
          onChange={(event) => choose(event.target.value === 'trash' ? 'trash' : 'folder')}
        >
          <option value="folder">{t('destination.folder')}</option>
          <option value="trash">{t('destination.trash')}</option>
        </select>
      </label>
      <TrashConfirmDialog
        open={confirming}
        onConfirm={confirm}
        onCancel={() => setConfirming(false)}
      />
    </>
  );
}

/** {@link DestinationPicker} for the remembered destination, shared by all windows. */
export function DestinationSetting() {
  const [destination, setDestination] = useRejectDestination();
  return <DestinationPicker destination={destination} onChange={setDestination} />;
}
