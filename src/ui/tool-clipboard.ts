export async function copyText(field: HTMLTextAreaElement, status: HTMLElement): Promise<void> {
  field.focus();
  field.select();
  try {
    if (navigator.clipboard?.writeText) {
      await navigator.clipboard.writeText(field.value);
      status.textContent = "복사했습니다.";
      return;
    }
  } catch {
    // The selected text remains available for WebViews without clipboard access.
  }
  const copied = document.execCommand?.("copy");
  status.textContent = copied ? "복사했습니다." : "텍스트를 선택했습니다. 복사 단축키를 사용하세요.";
}
