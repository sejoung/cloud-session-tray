import { t } from "./i18n";
import type { SharedProfile } from "../domain";
import { bindSubmit, element, escape, fact, text } from "./html";
import { shellQuote } from "./shell-commands";
import { copyText } from "./tool-clipboard";

function details(shared: SharedProfile): string {
  const auth = shared.authentication;
  if (auth.kind === "iamMfa") {
    return fact(t("share.recipientSource"), auth.sourceHint || t("share.chooseSource"), true)
      + fact(t("common.duration"), t("share.seconds", { seconds: auth.duration ?? 43200 }))
      + fact(t("common.authentication"), "IAM MFA");
  }
  return fact(t("common.authentication"), "IAM Identity Center") + fact(t("common.ssoRegion"), auth.settings.ssoRegion)
    + fact(t("common.awsAccount"), auth.settings.accountId, true) + fact(t("share.role"), auth.settings.roleName, true)
    + fact(t("share.startUrl"), auth.settings.startUrl, true, true);
}

function profileGuide(shared: SharedProfile): string {
  const lines = [shared.label, t("share.guideProfile", { profile: shared.profile })];
  if (shared.region) lines.push(t("share.guideRegion", { region: shared.region }));
  const auth = shared.authentication;
  if (auth.kind === "iamMfa") {
    lines.push(
      t("share.guideSource", { source: auth.sourceHint || t("share.guideOwnSource") }),
      t("share.guideTarget", { profile: shared.profile }),
      t("share.guideDuration", { seconds: auth.duration ?? 43200 }),
      t("share.guideMfa"),
      `aws configure mfa-login --profile=${shellQuote(auth.sourceHint || "default")} --update-profile=${shellQuote(shared.profile)} --duration-seconds=${auth.duration ?? 43200}`,
      t("share.guideTools", { profile: shared.profile }),
    );
  } else {
    const { startUrl, ssoRegion, accountId, roleName } = auth.settings;
    lines.push(
      `Identity Center ${startUrl}`, t("share.guideSsoRegion", { region: ssoRegion }), t("share.guideSsoIdentity", { account: accountId, role: roleName }),
      t("share.guideSsoLogin", { profile: shared.profile }),
    );
  }
  if (shared.task.trim()) lines.push("", t("common.task"), shared.task.trim());
  return lines.join("\n");
}

function profileInput(shared: SharedProfile): string {
  const label = shared.authentication.kind === "iamMfa" ? t("common.targetProfile") : t("common.profileName");
  return `<label class="field">${label}
    <input name="profile" required maxlength="256" value="${escape(shared.profile)}" />
  </label>`;
}

export function showShareDialog(
  dialog: HTMLDialogElement, shared: SharedProfile, save: (value: SharedProfile) => Promise<boolean>,
): void {
  dialog.innerHTML = `<form>
    <div class="dialog-heading"><h2 id="workflow-title" tabindex="-1" autofocus>${text("share.title")}</h2></div>
    <div class="dialog-body">
      <p class="notice">${text("share.credentialsExcluded")}</p>
      <dl class="facts">${details(shared)}${fact(t("common.region"), shared.region ?? t("common.notSet"))}</dl>
      ${profileInput(shared)}
      <label class="field">${text("common.optionalTask")}
        <textarea name="task" rows="2" maxlength="4000" aria-describedby="share-note-privacy">${escape(shared.task)}</textarea>
      </label>
      <p class="field-hint" id="share-note-privacy">${text("share.notePrivacy")}</p>
      <label class="field">${text("share.instructions")}
        <textarea data-guide readonly rows="4"></textarea>
      </label>
    </div>
    <div class="dialog-footer">
      <p class="muted" role="status" data-copy-status></p>
      <p class="form-error" role="alert" hidden></p>
      <div class="actions dialog-actions">
        <button class="button primary" type="submit">${text("share.export")}</button>
        <button class="button" type="button" data-copy>${text("share.copy")}</button>
        <button class="button subtle" type="button" data-close="workflow">${text("common.close")}</button>
      </div>
    </div>
  </form>`;
  const form = element<HTMLFormElement>(dialog, "form");
  const profile = element<HTMLInputElement>(form, '[name="profile"]');
  const task = element<HTMLTextAreaElement>(form, '[name="task"]');
  const guide = element<HTMLTextAreaElement>(form, "[data-guide]");
  const status = element<HTMLElement>(form, "[data-copy-status]");
  const preview = () => ({ ...shared, profile: profile.value.trim(), task: task.value.trim() });
  const refresh = () => { guide.value = profileGuide(preview()); };
  form.addEventListener("input", refresh);
  element(form, "[data-copy]").addEventListener("click", () => {
    if (profile.reportValidity()) void copyText(guide, status);
  });
  bindSubmit(form, async () => {
    if (await save(preview())) status.textContent = t("share.exported");
  });
  refresh();
  if (!dialog.open) dialog.showModal();
}

export function showImportDialog(
  dialog: HTMLDialogElement, shared: SharedProfile, apply: (value: SharedProfile) => Promise<void>,
): void {
  dialog.innerHTML = `<form>
    <div class="dialog-heading"><h2 id="workflow-title">${text("share.importTitle")}</h2></div>
    <div class="dialog-body">
      <p class="dialog-summary">${escape(shared.label)}</p>
      ${profileInput(shared)}
      <dl class="facts">${details(shared)}${fact(t("common.region"), shared.region ?? t("common.notSet"))}</dl>
      ${shared.task ? `<section class="task-note"><h3>${text("common.task")}</h3><p>${escape(shared.task)}</p></section>` : ""}
    </div>
    <div class="dialog-footer">
      <p class="form-error" role="alert" hidden></p>
      <div class="actions dialog-actions">
        <button class="button primary" type="submit">${shared.authentication.kind === "sso" ? t("share.importProfile") : t("share.openIam")}</button>
        <button class="button" type="button" data-close="workflow">${text("common.cancel")}</button>
      </div>
    </div>
  </form>`;
  const form = element<HTMLFormElement>(dialog, "form");
  bindSubmit(form, async () => {
    const profile = element<HTMLInputElement>(form, '[name="profile"]').value.trim();
    await apply({ ...shared, profile });
    if (dialog.open) dialog.close();
  });
}
