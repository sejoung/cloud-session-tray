import { t } from "../../shared/i18n";
import { profileName, type Login, type Profile, type SessionDraft } from "../../shared/domain";
import { editorDefaults, suggestTarget, type EditorSelection } from "./editor-suggestions";
import { bindSubmit, element, escape, text } from "../../shared/html";
import { bindMfaLookup } from "./mfa-devices";
import { bindRegionField, regionField } from "./region-field";

type EditorOptions = EditorSelection & {
  items: Profile[];
  managed: ReadonlySet<string>;
  lookupMfaDevices: (source: string) => Promise<string[]>;
  onSave: (draft: SessionDraft) => Promise<void>;
};

type Defaults = ReturnType<typeof editorDefaults>;

function profileOption(profile: Profile, current?: string): string {
  const label = [profile.name, profile.region, profile.preferred ? t("common.defaultProfile") : null].filter(Boolean).join(" · ");
  return `<option value="${escape(profile.name)}" ${profile.name === current ? "selected" : ""}>${escape(label)}</option>`;
}

function iamFields(items: Profile[], defaults: Defaults): string {
  const { sources, source, target, device, duration, region } = defaults;
  const targets = items.filter((profile) => profile.targetAllowed);
  const setup = `<div class="inline-help">
    <p>${text("editor.sourceRequired")}</p>
    <button type="button" class="button subtle" data-docs="iam">${text("editor.profileDocs")}</button>
  </div>`;
  return `<fieldset data-auth="iamMfa">
    ${sources.length === 0 ? setup : ""}
    <label class="field">${text("common.sourceProfile")}
      <select name="source" required>
        <option value="">${text("editor.selectSource")}</option>
        ${sources.map((profile) => profileOption(profile, source)).join("")}
      </select>
    </label>
    <button class="button subtle registration-link" type="button" data-action="editor-register-iam">${text("common.openIamRegistration")}</button>
    <label class="field">${text("common.targetProfile")}
      <input name="target" list="targets" required maxlength="256" placeholder="${text("editor.targetExample")}" value="${escape(target)}" />
      <datalist id="targets">${targets.map((profile) => profileOption(profile)).join("")}</datalist>
    </label>
    <p class="field-hint">${text("editor.targetHelp")}</p>
    ${regionField(t("editor.targetRegion"), t("editor.inheritedRegion"), region ?? "")}
    <label class="field">${text("editor.mfaDevice")}
      <input name="device" required maxlength="256" spellcheck="false" value="${escape(device)}" />
    </label>
    <div class="mfa-lookup">
      <button type="button" class="button" data-device-lookup aria-describedby="mfa-device-status">${text("editor.findMfa")}</button>
      <select data-device-options aria-label="${text("editor.foundMfa")}" hidden></select>
      <p id="mfa-device-status" class="muted" data-device-status role="status"></p>
    </div>
    <label class="field">${text("editor.durationSeconds")}
      <input name="duration" type="number" min="900" max="129600" step="1" placeholder="${text("editor.cliDefault")}"
        value="${duration ?? ""}" />
    </label>
  </fieldset>`;
}

function ssoFields({ sso, ssoProfile }: Defaults): string {
  return `<fieldset data-auth="sso">
    <label class="field">${text("editor.ssoProfile")}
      <select name="profile" required>
        <option value="">${text("editor.selectSso")}</option>
        ${sso.map((profile) => profileOption(profile, ssoProfile)).join("")}
      </select>
    </label>
    <button class="button sso-wizard" type="button" data-action="editor-wizard" aria-describedby="sso-wizard-help">${text("common.ssoWizard")}</button>
    <p class="field-hint" id="sso-wizard-help">${text("editor.ssoHelp")}</p>
  </fieldset>`;
}

function editorHtml(items: Profile[], defaults: Defaults, { session, preset }: EditorSelection): string {
  const unsupported = items.filter((profile) => profile.kind === "unsupported");
  return `<form id="session-form">
    <div class="dialog-heading">
      <h2 id="editor-title">${session ? t("editor.editProfile") : t("editor.addProfile")}</h2>
      <button type="button" class="icon-button" data-close="editor" aria-label="${text("common.close")}">×</button>
    </div>
    <div class="dialog-body">
      <label class="field">${text("common.authentication")}
        <select name="kind">
          <option value="iamMfa" ${defaults.kind === "iamMfa" ? "selected" : ""}>IAM MFA</option>
          <option value="sso" ${defaults.kind === "sso" ? "selected" : ""}>IAM Identity Center</option>
        </select>
      </label>
      ${iamFields(items, defaults)}
      ${ssoFields(defaults)}
      <label class="field">${text("editor.displayName")}
        <input name="label" placeholder="${text("editor.useProfileName")}" maxlength="64" value="${escape(session?.label ?? preset?.label ?? "")}" />
      </label>
      <label class="field">${text("common.optionalTask")}
        <textarea name="task" rows="2" maxlength="4000">${escape(preset?.task ?? session?.task ?? "")}</textarea>
      </label>
      ${unsupported.length ? `<details class="unsupported"><summary>${text("editor.unsupportedCount", { count: unsupported.length })}</summary>
        <p>${unsupported.map((profile) => escape(profile.name)).join(", ")}</p></details>` : ""}
    </div>
    <div class="dialog-footer">
      <p class="form-error" role="alert" hidden></p>
      <div class="actions dialog-actions">
        <button class="button primary" type="submit">${text("common.save")}</button>
        <button class="button" type="button" data-close="editor">${text("common.cancel")}</button>
        ${session ? `<button class="button subtle danger" type="button" data-action="remove">${text("editor.removeFromList")}</button>` : ""}
      </div>
    </div>
  </form>`;
}

function bindAuthMode(form: HTMLFormElement): void {
  const mode = element<HTMLSelectElement>(form, 'select[name="kind"]');
  const updateFields = () => {
    for (const fieldset of form.querySelectorAll<HTMLFieldSetElement>("fieldset")) {
      fieldset.disabled = fieldset.hidden = fieldset.dataset.auth !== mode.value;
    }
  };
  mode.addEventListener("change", updateFields);
  updateFields();
}

function offer(input: HTMLInputElement, suggestion: string): void {
  if (!input.value || input.value === input.dataset.suggestion) input.value = suggestion;
  input.dataset.suggestion = suggestion;
}

function bindSuggestions(form: HTMLFormElement, options: EditorOptions, defaults: Defaults): void {
  const source = element<HTMLSelectElement>(form, 'select[name="source"]');
  const device = element<HTMLInputElement>(form, 'input[name="device"]');
  const target = element<HTMLInputElement>(form, 'input[name="target"]');
  if (!options.session && !options.preset) {
    target.dataset.suggestion = defaults.target;
    device.dataset.suggestion = defaults.device;
  }
  source.addEventListener("change", () => {
    const original = defaults.sources.find((profile) => profile.name === source.value);
    offer(device, original?.mfaDevice ?? "");
    offer(target, suggestTarget(source.value, options.items, options.managed));
  });
  bindMfaLookup(form, source, options.lookupMfaDevices);
}

function readSessionDraft(form: HTMLFormElement, id: string | null, region: string): SessionDraft {
  const entries = new FormData(form);
  const value = (name: string) => String(entries.get(name) ?? "").trim();
  const duration = value("duration");
  const task = value("task");
  const login: Login = value("kind") === "sso" ? { kind: "sso", profile: value("profile") } : {
    kind: "iamMfa", source: value("source"), target: value("target"), device: value("device"),
    duration: duration ? Number(duration) : null,
    ...(region ? { region } : {}),
  };
  return { id, label: value("label") || profileName(login), login, ...(task ? { task } : {}) };
}

export function showEditor(dialog: HTMLDialogElement, options: EditorOptions): void {
  const { session, items, managed, onSave } = options;
  const defaults = editorDefaults(items, managed, options);
  dialog.innerHTML = editorHtml(items, defaults, options);
  const form = element<HTMLFormElement>(dialog, "form");
  bindAuthMode(form);
  bindSuggestions(form, options, defaults);
  const readRegion = bindRegionField(form);
  bindSubmit(form, async () => {
    await onSave(readSessionDraft(form, session?.id ?? null, readRegion()));
    if (dialog.open) dialog.close();
  });
  if (!dialog.open) dialog.showModal();
}
