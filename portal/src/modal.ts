const overlay = document.getElementById('modal-overlay')! as HTMLDivElement;
const messageElement = document.getElementById(
  'modal-message'
)! as HTMLParagraphElement;
const confirmButton = document.getElementById(
  'modal-confirm'
)! as HTMLButtonElement;
const cancelButton = document.getElementById(
  'modal-cancel'
)! as HTMLButtonElement;

const FOCUSABLE = [cancelButton, confirmButton];

export function confirm(message: string): Promise<boolean> {
  return new Promise((resolve) => {
    const previousFocus = document.activeElement as HTMLElement | null;

    messageElement.textContent = message;
    overlay.classList.add('active');
    cancelButton.focus();

    function cleanup(isConfirmed: boolean): void {
      overlay.classList.remove('active');
      confirmButton.removeEventListener('click', onConfirm);
      cancelButton.removeEventListener('click', onCancel);
      overlay.removeEventListener('click', onOverlay);
      document.removeEventListener('keydown', onKey);
      previousFocus?.focus();
      resolve(isConfirmed);
    }

    function onConfirm(): void {
      cleanup(true);
    }

    function onCancel(): void {
      cleanup(false);
    }

    function onOverlay(event: Event): void {
      if (event.target === overlay) cleanup(false);
    }

    function onKey(event: KeyboardEvent): void {
      if (event.key === 'Escape') {
        cleanup(false);
        return;
      }

      if (event.key !== 'Tab') return;

      const index = FOCUSABLE.indexOf(
        document.activeElement as HTMLButtonElement
      );
      if (event.shiftKey) {
        if (index <= 0) {
          event.preventDefault();
          FOCUSABLE[FOCUSABLE.length - 1].focus();
        }
      } else {
        if (index >= FOCUSABLE.length - 1) {
          event.preventDefault();
          cancelButton.focus();
        }
      }
    }

    confirmButton.addEventListener('click', onConfirm);
    cancelButton.addEventListener('click', onCancel);
    overlay.addEventListener('click', onOverlay);
    document.addEventListener('keydown', onKey);
  });
}
