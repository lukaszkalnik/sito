import { useId, type ReactNode } from 'react';

import './controls.css';

/**
 * A control with a small caption above it. `children` receives the id to put
 * on the control, so the caption is the control's accessible name.
 */
export function Field({
  caption,
  className,
  title,
  children,
}: {
  caption: string;
  /** Extra class on the field, e.g. for a coloured state. */
  className?: string;
  /** Tooltip for the whole field. */
  title?: string;
  children: (id: string) => ReactNode;
}) {
  const id = useId();
  return (
    <div className={className === undefined ? 'field' : `field ${className}`} title={title}>
      <label htmlFor={id} className="field-caption">
        {caption}
      </label>
      {children(id)}
    </div>
  );
}
