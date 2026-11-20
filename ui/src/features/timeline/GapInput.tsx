import { useEffect, useRef, useState } from 'react';
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

/** How long typing must pause before a typed gap regroups the series, in ms. */
const TYPING_PAUSE_MS = 300;

/**
 * The seconds as a number input. A valid value regroups once typing pauses;
 * an empty or out-of-range one waits, and leaving the field applies the
 * value at once, clamped to the range.
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
  const { text, type, finish } = useTypedGap(gap, onChange);
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
      onChange={(event) => type(event.target.value)}
      onBlur={finish}
    />
  );
}

/**
 * The text typed into the gap input, applied after a pause in typing.
 * `text` is `null` while the field shows the gap itself.
 */
function useTypedGap(gap: number, onChange: (gap: number) => void) {
  const [draft, setDraft] = useState<string | null>(null);
  // Any change of the gap (a typed value applied, a step with − or +) ends
  // the typing, so stale text never comes back.
  const [shownGap, setShownGap] = useState(gap);
  if (shownGap !== gap) {
    setShownGap(gap);
    setDraft(null);
  }
  const text = draft;
  const timer = useRef<number | undefined>(undefined);
  const latest = useRef({ gap, onChange });
  useEffect(() => {
    latest.current = { gap, onChange };
  });
  useEffect(() => () => window.clearTimeout(timer.current), []);

  const type = (typed: string) => {
    window.clearTimeout(timer.current);
    setDraft(typed);
    const seconds = typedGap(typed);
    if (seconds === null || seconds === gap) return;
    timer.current = window.setTimeout(() => {
      // A step with − or + since then wins over the typed value.
      if (latest.current.gap === gap) latest.current.onChange(seconds);
    }, TYPING_PAUSE_MS);
  };
  const finish = () => {
    window.clearTimeout(timer.current);
    setDraft(null);
    if (text === null) return;
    const finished = finishedGap(text, gap);
    if (finished !== gap) onChange(finished);
  };
  return { text, type, finish };
}
