import { act, fireEvent, render, screen } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

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

/** Types `text` as the whole field content, as each key press leaves it. */
function typeInto(input: HTMLElement, text: string) {
  fireEvent.change(input, { target: { value: text } });
}

/** Lets the pause after typing pass. */
function pause(ms = 300) {
  act(() => vi.advanceTimersByTime(ms));
}

describe('GapInput', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows the gap in seconds with its range', () => {
    const { input } = renderInput(10);

    expect(input).toHaveValue(10);
    expect(input).toHaveAttribute('min', '1');
    expect(input).toHaveAttribute('max', '60');
    expect(input).toHaveAttribute('aria-valuetext', '10 s');
    expect(screen.getByText('s')).toBeInTheDocument();
  });

  it('applies a typed value only once typing pauses', () => {
    const { input, onChange } = renderInput(10);

    typeInto(input, '');
    typeInto(input, '2');
    typeInto(input, '25');
    act(() => vi.advanceTimersByTime(299));
    expect(onChange).not.toHaveBeenCalled();
    expect(input).toHaveValue(25);

    pause(1);
    expect(onChange.mock.calls).toEqual([[25]]);
    expect(input).toHaveValue(25);
  });

  it('applies a typed value at once when leaving the field', () => {
    const { input, onChange } = renderInput(10);

    fireEvent.change(input, { target: { value: '25' } });
    fireEvent.blur(input);
    expect(onChange.mock.calls).toEqual([[25]]);

    pause();
    expect(onChange.mock.calls).toEqual([[25]]);
  });

  it('does not apply a typed value that equals the gap', () => {
    const { input, onChange } = renderInput(10);

    fireEvent.change(input, { target: { value: '10' } });
    pause();
    fireEvent.blur(input);

    expect(onChange).not.toHaveBeenCalled();
  });

  it('waits on an empty field and restores the gap when leaving it', () => {
    const { input, onChange } = renderInput(10);

    typeInto(input, '');
    expect(input).toHaveValue(null);
    pause();
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
    pause();
    expect(onChange).not.toHaveBeenCalled();
    fireEvent.blur(input);

    expect(onChange).toHaveBeenLastCalledWith(expected);
    expect(input).toHaveValue(expected);
  });

  it('steps with the buttons at once, dropping a typed value still waiting', () => {
    const { input, onChange } = renderInput(10);
    fireEvent.change(input, { target: { value: '30' } });

    fireEvent.click(screen.getByRole('button', { name: 'Zwiększ odstęp' }));
    expect(onChange.mock.calls).toEqual([[11]]);
    expect(input).toHaveValue(11);

    pause();
    expect(onChange.mock.calls).toEqual([[11]]);
    fireEvent.click(screen.getByRole('button', { name: 'Zmniejsz odstęp' }));
    expect(input).toHaveValue(10);
  });

  it('applies the last typed value after typing again within the pause', () => {
    const { input, onChange } = renderInput(10);

    fireEvent.change(input, { target: { value: '3' } });
    act(() => vi.advanceTimersByTime(200));
    fireEvent.change(input, { target: { value: '35' } });
    act(() => vi.advanceTimersByTime(200));
    expect(onChange).not.toHaveBeenCalled();

    pause(100);
    expect(onChange.mock.calls).toEqual([[35]]);
  });
});
