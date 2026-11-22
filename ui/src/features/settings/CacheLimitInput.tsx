import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Field } from '../../controls/Field';
import { finishedLimit, MIN_LIMIT_MB } from './cacheLimit';

/**
 * The space thumbnails may take, in MB. The typed value applies when the
 * field is left or Enter is pressed, at least {@link MIN_LIMIT_MB}.
 */
export function CacheLimitInput({
  limitMb,
  onChange,
}: {
  limitMb: number;
  onChange: (mb: number) => void;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<string | null>(null);
  const finish = () => {
    if (draft === null) return;
    setDraft(null);
    const mb = finishedLimit(draft, limitMb);
    if (mb !== limitMb) onChange(mb);
  };
  return (
    <Field caption={t('cache.limit')} title={t('cache.limitHint', { min: MIN_LIMIT_MB })}>
      {(id) => (
        <div className="cache-limit">
          <input
            id={id}
            type="number"
            inputMode="numeric"
            min={MIN_LIMIT_MB}
            step={1}
            value={draft ?? String(limitMb)}
            onChange={(event) => setDraft(event.target.value)}
            onBlur={finish}
            onKeyDown={(event) => {
              if (event.key === 'Enter') finish();
            }}
          />
          <span aria-hidden="true">{t('cache.limitUnit')}</span>
        </div>
      )}
    </Field>
  );
}
