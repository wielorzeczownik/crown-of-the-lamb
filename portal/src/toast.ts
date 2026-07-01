const toastContainer: { element: HTMLDivElement | undefined } = {
  element: undefined,
};

function getToastContainer(): HTMLDivElement {
  if (!toastContainer.element) {
    toastContainer.element = document.createElement('div');
    toastContainer.element.id = 'toast-container';
    document.body.append(toastContainer.element);
  }
  return toastContainer.element;
}

export function showToast(isOk: boolean, message: string): void {
  const container = getToastContainer();
  const toast = document.createElement('div');
  toast.className = 'toast ' + (isOk ? 'toast--ok' : 'toast--error');
  toast.textContent = message;
  container.append(toast);
  setTimeout(() => {
    toast.classList.add('toast--out');
    setTimeout(() => toast.remove(), 280);
  }, 2200);
}
