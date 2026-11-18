import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it, vi } from 'vitest';

import '../../i18n';
import type { EditOption } from './editOptions';
import { SeriesMenu, type OpenMenu } from './SeriesMenu';

const options: EditOption[] = [
  { action: 'cut', edit: { kind: 'cut', photo: 1 } },
  { action: 'remove', edit: { kind: 'remove', photo: 1 } },
  { action: 'ungroup', edit: { kind: 'ungroup', photo: 0 } },
];

const menu: OpenMenu = { x: 10, y: 20, options };

function renderMenu() {
  const onChoose = vi.fn();
  const onClose = vi.fn();
  render(
    <>
      <button type="button">Elsewhere</button>
      <SeriesMenu menu={menu} onChoose={onChoose} onClose={onClose} />
    </>,
  );
  return { onChoose, onClose };
}

const item = (name: string) => screen.getByRole('menuitem', { name });

describe('SeriesMenu', () => {
  it('shows nothing while closed', () => {
    render(<SeriesMenu menu={null} onChoose={vi.fn()} onClose={vi.fn()} />);

    expect(screen.queryByRole('menu')).not.toBeInTheDocument();
  });

  it('lists the edits and focuses the first', () => {
    renderMenu();

    expect(screen.getByRole('menu', { name: 'Edycja serii' })).toBeInTheDocument();
    expect(screen.getAllByRole('menuitem').map((i) => i.textContent)).toEqual([
      'Podziel serię przed tym zdjęciem',
      'Wyjmij z serii',
      'Rozdziel serię',
    ]);
    expect(item('Podziel serię przed tym zdjęciem')).toHaveFocus();
  });

  it('moves through the edits with the arrow keys, wrapping around', () => {
    renderMenu();

    fireEvent.keyDown(window, { key: 'ArrowDown' });
    expect(item('Wyjmij z serii')).toHaveFocus();
    fireEvent.keyDown(window, { key: 'ArrowUp' });
    fireEvent.keyDown(window, { key: 'ArrowUp' });
    expect(item('Rozdziel serię')).toHaveFocus();
  });

  it('makes the chosen edit', () => {
    const { onChoose } = renderMenu();

    fireEvent.click(item('Wyjmij z serii'));

    expect(onChoose).toHaveBeenCalledWith(options[1]);
  });

  it('closes with Escape, giving the focus back', () => {
    const { onClose } = renderMenu();

    fireEvent.keyDown(window, { key: 'Escape' });

    expect(onClose).toHaveBeenCalledWith(true);
  });

  it('closes on a press outside it or when the focus leaves it', () => {
    const { onClose } = renderMenu();

    fireEvent.pointerDown(screen.getByRole('button', { name: 'Elsewhere' }));
    expect(onClose).toHaveBeenLastCalledWith(false);

    onClose.mockClear();
    fireEvent.blur(item('Wyjmij z serii'), {
      relatedTarget: screen.getByRole('button', { name: 'Elsewhere' }),
    });
    expect(onClose).toHaveBeenCalledWith(false);
  });
});
