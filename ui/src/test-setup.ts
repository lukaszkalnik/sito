import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

// Testing Library only auto-cleans when Vitest globals are enabled.
afterEach(() => {
  cleanup();
});

// jsdom does no layout, so it has no scrolling into view; tests spy on this stub.
if (!('scrollIntoView' in Element.prototype)) {
  Object.assign(Element.prototype, { scrollIntoView() {} });
}

// jsdom has no modal dialogs; enough of them for components that use one.
if (!('showModal' in HTMLDialogElement.prototype)) {
  Object.assign(HTMLDialogElement.prototype, {
    showModal(this: HTMLDialogElement) {
      this.open = true;
    },
    close(this: HTMLDialogElement) {
      if (!this.open) return;
      this.open = false;
      this.dispatchEvent(new Event('close'));
    },
  });
}
