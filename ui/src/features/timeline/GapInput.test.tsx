import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { useState } from 'react';
import { describe, expect, it, vi } from 'vitest';

import '../../i18n';
import { GapInput } from './GapInput';

/** The input with its gap in state, as the timeline holds it; `onChange` sees every change. */
function Harness({ initial, onChange }: { initial: number; onChange: (gap: number) => void }) {
  const [gap, setGap] = useState(initial);
  return (
    <GapInput
      gap={gap}
      onChange={(next) => {
        onChange(next);
        setGap(next);
      }}
    />
  );
}

function renderInput(initial: number) {
  const onChange = vi.fn<(gap: number) => void>();
  render(<Harness initial={initial} onChange={onChange} />);
  return { onChange, input: screen.getByRole('spinbutton', { name: 'Odstęp w serii' }) };
}

describe('GapInput', () => {
  it('shows the gap in seconds with its range', () => {
    const { input } = renderInput(10);

    expect(input).toHaveValue(10);
    expect(input).toHaveAttribute('min', '1');
    expect(input).toHaveAttribute('max', '60');
    expect(input).toHaveAttribute('aria-valuetext', '10 s');
    expect(screen.getByText('s')).toBeInTheDocument();
  });

  it('applies each valid value while typing', async () => {
    const { input, onChange } = renderInput(10);

    await userEvent.clear(input);
    await userEvent.type(input, '25');

    expect(onChange.mock.calls).toEqual([[2], [25]]);
    expect(input).toHaveValue(25);
  });

  it('waits on an empty field and restores the gap when leaving it', async () => {
    const { input, onChange } = renderInput(10);

    await userEvent.clear(input);
    expect(input).toHaveValue(null);
    fireEvent.blur(input);

    expect(onChange).not.toHaveBeenCalled();
    expect(input).toHaveValue(10);
  });

  it.each([
    ['90', 60],
    ['0', 1],
  ])('clamps %s typed out of range to %i when leaving the field', (text, expected) => {
    const { input, onChange } = renderInput(10);

    fireEvent.change(input, { target: { value: text } });
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.blur(input);

    expect(onChange).toHaveBeenLastCalledWith(expected);
    expect(input).toHaveValue(expected);
  });

  it('steps with the buttons, also after typing', async () => {
    const { input, onChange } = renderInput(10);
    fireEvent.change(input, { target: { value: '30' } });

    await userEvent.click(screen.getByRole('button', { name: 'Zwiększ odstęp' }));

    expect(onChange).toHaveBeenLastCalledWith(31);
    expect(input).toHaveValue(31);
    await userEvent.click(screen.getByRole('button', { name: 'Zmniejsz odstęp' }));
    expect(input).toHaveValue(30);
  });
});
