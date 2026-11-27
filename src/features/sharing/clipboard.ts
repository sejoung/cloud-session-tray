import { t } from "../../shared/i18n";
export async function copyText(field: HTMLTextAreaElement, status: HTMLElement): Promise<void> {
  field.focus();
  field.select();
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(field.value);
      status.textContent = t("clipboard.copied");
      return;
    }
  } catch {
    // The selected text remains available for WebViews without clipboard access.
  }
  const copied = document.execCommand?.("copy");
  status.textContent = copied ? t("clipboard.copied") : t("clipboard.manual");
}
