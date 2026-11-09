import { useTranslation } from 'react-i18next';

import { Field } from '../../controls/Field';
import type { TimelineMode } from './layout';
import { parseView, VIEW_LABELS, VIEWS } from './views';

/** „Widok”: chooses which photos the timeline shows. */
export function ViewPicker({
  mode,
  onChange,
}: {
  mode: TimelineMode;
  onChange: (mode: TimelineMode) => void;
}) {
  const { t } = useTranslation();
  const choose = (value: string) => {
    const view = parseView(value);
    if (view !== null) onChange(view);
  };
  return (
    <Field caption={t('view.label')}>
      {(id) => (
        <select
          id={id}
          className="control-select"
          value={mode}
          onChange={(event) => choose(event.target.value)}
        >
          {VIEWS.map((view) => (
            <option key={view} value={view}>
              {t(VIEW_LABELS[view])}
            </option>
          ))}
        </select>
      )}
    </Field>
  );
}
