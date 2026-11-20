import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Field } from '../../controls/Field';
import { finishedGap, MAX_GAP, MIN_GAP, typedGap } from './gapSetting';
import './GapInput.css';

/** Number input for the largest time gap between neighbouring photos of a series. */
export function GapInput({ gap, onChange }: { gap: number; onChange: (gap: number) => void }) {
  const { t } = useTranslation();
  return (
    <Field caption={t('gap.label')} title={t('gap.hint')}>
      {(id) => <GapControls id={id} gap={gap} onChange={onChange} />}
    </Field>
  );
}

/** `[−] [10] s [+]`, with the number input's id `id`. */
function GapControls({
  id,
  gap,
  onChange,
}: {
  id: string;
  gap: number;
  onChange: (gap: number) => void;
}) {
  const { t } = useTranslation();
  return (
    <div className="timeline-gap">
      <button
        type="button"
        className="control-icon-button"
        aria-label={t('gap.decrease')}
        disabled={gap <= MIN_GAP}
        onClick={() => onChange(gap - 1)}
      >
        −
      </button>
      <SecondsInput id={id} gap={gap} onChange={onChange} />
      <span className="timeline-gap-unit" aria-hidden="true">
        {t('gap.unit')}
      </span>
      <button
        type="button"
        className="control-icon-button"
        aria-label={t('gap.increase')}
        disabled={gap >= MAX_GAP}
        onClick={() => onChange(gap + 1)}
      >
        +
      </button>
    </div>
  );
}

/**
 * The seconds as a number input. A valid value regroups while typing; an
 * empty or out-of-range one waits, and leaving the field clamps it.
 */
function SecondsInput({
  id,
  gap,
  onChange,
}: {
  id: string;
  gap: number;
  onChange: (gap: number) => void;
}) {
  const { t } = useTranslation();
  // The text being typed and the gap it stands for. It is shown only while
  // that is still the gap, so a step with − or + replaces it.
  const [draft, setDraft] = useState<{ text: string; gap: number } | null>(null);
  const text = draft?.gap === gap ? draft.text : null;
  const finish = () => {
    setDraft(null);
    if (text === null) return;
    const finished = finishedGap(text, gap);
    if (finished !== gap) onChange(finished);
  };
  return (
    <input
      id={id}
      type="number"
      className="timeline-gap-input"
      inputMode="numeric"
      min={MIN_GAP}
      max={MAX_GAP}
      step={1}
      value={text ?? String(gap)}
      aria-valuetext={t('gap.value', { seconds: gap })}
      onChange={(event) => {
        const typed = typedGap(event.target.value);
        setDraft({ text: event.target.value, gap: typed ?? gap });
        if (typed !== null && typed !== gap) onChange(typed);
      }}
      onBlur={finish}
    />
  );
}
