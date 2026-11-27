import { profileName, type Session } from "../domain";
import { bindSubmit, element, escape, text } from "./html";

export function showOtp(
  dialog: HTMLDialogElement,
  session: Session,
  submitCode: (code: string) => Promise<void>,
): void {
  dialog.innerHTML = `<form>
    <div class="dialog-heading">
      <h2 id="otp-title">${text("confirmation.renewMfa")}</h2>
      <button type="button" class="icon-button" data-close="otp" aria-label="${text("common.close")}">×</button>
    </div>
    <div class="dialog-body">
      <p class="dialog-summary">${escape(session.label)} <span class="mono">${escape(profileName(session.login))}</span></p>
      <label class="field">${text("confirmation.mfaCode")}
        <input type="password" inputmode="numeric" pattern="[0-9]{6}" minlength="6" maxlength="6"
          autocomplete="off" name="otp" required autofocus />
      </label>
    </div>
    <div class="dialog-footer">
      <p class="form-error" role="alert" hidden></p>
      <div class="actions dialog-actions">
        <button class="button primary" type="submit">${text("common.renew")}</button>
        <button class="button" type="button" data-close="otp">${text("common.cancel")}</button>
      </div>
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
    <div class="dialog-heading"><h2 id="confirm-title">${text("confirmation.removeSession")}</h2></div>
    <div class="dialog-body"><p class="dialog-summary">${text("confirmation.removeDescription", { label: session.label })}</p></div>
    <div class="dialog-footer actions dialog-actions">
      <button class="button" value="cancel" autofocus>${text("common.cancel")}</button>
      <button class="button danger" value="remove">${text("common.remove")}</button>
    </div>
  </form>`;
  dialog.returnValue = "";
  const result = new Promise<boolean>((resolve) => {
    dialog.addEventListener("close", () => resolve(dialog.returnValue === "remove"), { once: true });
  });
  dialog.showModal();
  return result;
}
