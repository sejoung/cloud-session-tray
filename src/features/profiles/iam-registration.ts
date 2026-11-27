import { t } from "../../shared/i18n";
import type { IamRegistration } from "../../shared/domain";
import { bindSubmit, element, text } from "../../shared/html";
import { bindRegionField, regionField } from "./region-field";

export function showIamRegistration(
  dialog: HTMLDialogElement,
  region: string,
  register: (registration: IamRegistration) => Promise<void>,
): void {
  if (dialog.open) return;
  dialog.innerHTML = `<form id="registration-form">
    <div class="dialog-heading">
      <h2 id="registration-title">${text("common.openIamRegistration")}</h2>
      <button type="button" class="icon-button" data-close="registration" aria-label="${text("common.close")}">×</button>
    </div>
    <div class="dialog-body">
      <p class="dialog-summary">${text("iam.description")}</p>
      <label class="field">${text("common.profileName")}
        <input name="name" required maxlength="256" autofocus />
      </label>
      <label class="field">${text("iam.accessKeyId")}
        <input name="accessKeyId" type="password" required minlength="16" maxlength="128" autocomplete="off" />
      </label>
      <label class="field">${text("iam.secretKey")}
        <input name="secretAccessKey" type="password" required minlength="16" maxlength="256" autocomplete="off" />
      </label>
      ${regionField(t("iam.defaultRegion"), t("iam.noRegion"), region)}
      <p class="field-hint">${text("iam.outputFormat")}</p>
    </div>
    <div class="dialog-footer">
      <p class="form-error" role="alert" hidden></p>
      <div class="actions dialog-actions">
        <button class="button primary" type="submit">${text("common.register")}</button>
        <button class="button" type="button" data-close="registration">${text("common.cancel")}</button>
      </div>
    </div>
  </form>`;

  const form = element<HTMLFormElement>(dialog, "form");
  const readRegion = bindRegionField(form);
  const accessKeyId = element<HTMLInputElement>(form, '[name="accessKeyId"]');
  const secretAccessKey = element<HTMLInputElement>(form, '[name="secretAccessKey"]');
  const closeButtons = Array.from(form.querySelectorAll<HTMLButtonElement>("[data-close]"));
  const setPending = (pending: boolean) => {
    for (const button of closeButtons) button.disabled = pending;
  };
  dialog.oncancel = (event) => {
    if (closeButtons.some((button) => button.disabled)) event.preventDefault();
  };
  bindSubmit(form, async () => {
    const registration = {
      name: element<HTMLInputElement>(form, '[name="name"]').value.trim(),
      region: readRegion(),
      accessKeyId: accessKeyId.value.trim(),
      secretAccessKey: secretAccessKey.value.trim(),
    };
    accessKeyId.value = "";
    secretAccessKey.value = "";
    setPending(true);
    try {
      await register(registration);
      if (dialog.open) dialog.close();
    } finally {
      setPending(false);
    }
  });
  dialog.addEventListener("close", () => dialog.replaceChildren(), { once: true });
  dialog.showModal();
}
