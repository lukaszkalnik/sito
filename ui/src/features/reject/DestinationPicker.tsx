import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Field } from '../../controls/Field';
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
      <div className="reject-destination-slot">
        <TrashLookSizer />
        <Field
          caption={t('destination.label')}
          className={destination === 'trash' ? 'reject-destination--trash' : undefined}
        >
          {(id) => (
            <div className="reject-destination">
              <DestinationIcon destination={destination} />
              <select
                id={id}
                className="control-select"
                value={destination}
                onChange={(event) => choose(event.target.value === 'trash' ? 'trash' : 'folder')}
              >
                <DestinationOptions />
              </select>
            </div>
          )}
        </Field>
      </div>
      <TrashConfirmDialog
        open={confirming}
        onConfirm={confirm}
        onCancel={() => setConfirming(false)}
      />
    </>
  );
}

function DestinationOptions() {
  const { t } = useTranslation();
  return (
    <>
      <option value="folder">{t('destination.folder')}</option>
      <option value="trash">{t('destination.trash')}</option>
    </>
  );
}

/**
 * An invisible copy of the picker in its bold trash look, stacked under the
 * real one. It keeps the picker as wide as its widest look, so switching the
 * destination does not shift the controls next to it.
 */
function TrashLookSizer() {
  const { t } = useTranslation();
  return (
    <div
      className="field reject-destination--trash reject-destination-sizer"
      aria-hidden="true"
      inert
    >
      <span className="field-caption">{t('destination.label')}</span>
      <div className="reject-destination">
        <DestinationIcon destination="trash" />
        <select className="control-select" tabIndex={-1} defaultValue="trash">
          <DestinationOptions />
        </select>
      </div>
    </div>
  );
}

/** {@link DestinationPicker} for the remembered destination, shared by all windows. */
export function DestinationSetting() {
  const [destination, setDestination] = useRejectDestination();
  return <DestinationPicker destination={destination} onChange={setDestination} />;
}
