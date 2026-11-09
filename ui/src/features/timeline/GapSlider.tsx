import { useId } from 'react';
import { useTranslation } from 'react-i18next';

import { MAX_GAP, MIN_GAP } from './gapSetting';
import './GapSlider.css';

/** Slider for the largest time gap between neighbouring photos of a series. */
export function GapSlider({ gap, onChange }: { gap: number; onChange: (gap: number) => void }) {
  const { t } = useTranslation();
  const id = useId();
  return (
    <div className="timeline-gap" title={t('gap.hint')}>
      <label htmlFor={id}>{t('gap.label')}</label>
      <button
        type="button"
        className="timeline-gap-step"
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
        className="timeline-gap-step"
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
