const dateFormat = new Intl.DateTimeFormat("ko-KR", { dateStyle: "medium", timeStyle: "short" });

export function escape(value: string): string {
  return value.replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;",
  })[character]!);
}

export function date(value: string | null): string {
  return value ? dateFormat.format(new Date(value)) : "알 수 없음";
}

export function notice(message: string, warning = false): string {
  return `<p class="notice ${warning ? "notice-warning" : ""}" role="status">${escape(message)}</p>`;
}

export function header(title: string, description: string, actions = ""): string {
  return `<header class="page-header"><div><h1>${title}</h1><p>${description}</p></div>${actions}</header>`;
}

export function fact(title: string, value: string, mono = false, full = false): string {
  return `<div class="${full ? "full" : ""}"><dt>${title}</dt><dd class="${mono ? "mono" : ""}">${escape(value)}</dd></div>`;
}

export function element<T extends Element>(root: ParentNode, selector: string): T {
  const found = root.querySelector<T>(selector);
  if (!found) throw new Error(`Missing UI element ${selector}`);
  return found;
}

export function errorMessage(error: unknown, fallback: string): string {
  return typeof error === "string" ? error : fallback;
}

export function reportFormError(form: HTMLFormElement, error: unknown): void {
  const message = element<HTMLElement>(form, ".form-error");
  message.textContent = errorMessage(error, "입력한 설정을 확인해 주세요.");
  message.hidden = false;
}

export function bindSubmit(form: HTMLFormElement, submit: () => Promise<void>, onError?: () => void): void {
  const button = element<HTMLButtonElement>(form, 'button[type="submit"]');
  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (button.disabled) return;
    button.disabled = true;
    try {
      await submit();
    } catch (error) {
      reportFormError(form, error);
      onError?.();
    } finally {
      button.disabled = false;
    }
  });
}
