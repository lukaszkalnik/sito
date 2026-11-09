import { useTranslation } from 'react-i18next';

import { Field } from '../../controls/Field';
import { MAX_GAP, MIN_GAP } from './gapSetting';
import './GapSlider.css';

/** Slider for the largest time gap between neighbouring photos of a series. */
export function GapSlider({ gap, onChange }: { gap: number; onChange: (gap: number) => void }) {
  const { t } = useTranslation();
  return (
    <Field caption={t('gap.label')} title={t('gap.hint')}>
      {(id) => <GapControls id={id} gap={gap} onChange={onChange} />}
    </Field>
  );
}

/** `[−] slider [+] 10 s`, with the slider's id `id`. */
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
      <input
        id={id}
        type="range"
        min={MIN_GAP}
        max={MAX_GAP}
        step={1}
        value={gap}
        aria-valuetext={t('gap.value', { seconds: gap })}
        onChange={(event) => onChange(Number(event.target.value))}
      />
      <button
        type="button"
        className="control-icon-button"
        aria-label={t('gap.increase')}
        disabled={gap >= MAX_GAP}
        onClick={() => onChange(gap + 1)}
      >
        +
      </button>
      <span className="timeline-gap-value" aria-hidden="true">
        {t('gap.value', { seconds: gap })}
      </span>
    </div>
  );
}
