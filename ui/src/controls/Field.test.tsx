import { render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';

import { Field } from './Field';

describe('Field', () => {
  it('names its control by the caption', () => {
    render(<Field caption="Widok">{(id) => <select id={id} />}</Field>);

    expect(screen.getByRole('combobox', { name: 'Widok' })).toBeInTheDocument();
  });

  it('puts the tooltip on the whole field', () => {
    render(
      <Field caption="Odstęp" title="Podpowiedź">
        {(id) => <input id={id} type="range" />}
      </Field>,
    );

    expect(screen.getByRole('slider', { name: 'Odstęp' }).parentElement).toHaveAttribute(
      'title',
      'Podpowiedź',
    );
  });
});
