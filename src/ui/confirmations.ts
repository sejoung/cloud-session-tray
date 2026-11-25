import { profileName, type Session } from "../domain";
import { bindSubmit, element, escape } from "./html";

export function showOtp(
  dialog: HTMLDialogElement,
  session: Session,
  submitCode: (code: string) => Promise<void>,
): void {
  dialog.innerHTML = `<form>
    <div class="dialog-heading">
      <h2 id="otp-title">IAM MFA 갱신</h2>
      <button type="button" class="icon-button" data-close="otp" aria-label="닫기">×</button>
    </div>
    <p class="dialog-summary">${escape(session.label)} <span class="mono">${escape(profileName(session.login))}</span></p>
    <label class="field">MFA 인증 코드
      <input type="password" inputmode="numeric" pattern="[0-9]{6}" minlength="6" maxlength="6"
        autocomplete="off" name="otp" required autofocus />
    </label>
    <p class="form-error" role="alert" hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="submit">갱신</button>
      <button class="button" type="button" data-close="otp">취소</button>
    </div>
  </form>`;
  const form = element<HTMLFormElement>(dialog, "form");
  const input = element<HTMLInputElement>(form, 'input[name="otp"]');
  bindSubmit(form, async () => {
    const code = input.value;
    input.value = "";
    await submitCode(code);
    if (dialog.open) dialog.close();
  }, () => input.focus());
  if (!dialog.open) dialog.showModal();
  input.focus();
}

export function confirmRemoval(dialog: HTMLDialogElement, session: Session): Promise<boolean> {
  dialog.innerHTML = `<form method="dialog">
    <div class="dialog-heading"><h2 id="confirm-title">세션 목록에서 제거</h2></div>
    <p class="dialog-summary">${escape(session.label)} 연결을 이 앱에서 제거합니다.</p>
    <div class="actions dialog-actions">
      <button class="button" value="cancel" autofocus>취소</button>
      <button class="button danger" value="remove">제거</button>
    </div>
  </form>`;
  dialog.returnValue = "";
  const result = new Promise<boolean>((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue === "remove"), { once: true });
  });
  dialog.showModal();
  return result;
}
