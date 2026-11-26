import { profileName, type Login, type Profile, type SessionDraft } from "../domain";
import { editorDefaults, suggestTarget, type EditorSelection } from "./editor-suggestions";
import { bindSubmit, element, escape } from "./html";
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
  const label = [profile.name, profile.region, profile.preferred ? "기본 프로필" : null].filter(Boolean).join(" · ");
  return `<option value="${escape(profile.name)}" ${profile.name === current ? "selected" : ""}>${escape(label)}</option>`;
}

function iamFields(items: Profile[], defaults: Defaults): string {
  const { sources, source, target, device, duration, region } = defaults;
  const targets = items.filter((profile) => profile.targetAllowed);
  const setup = `<div class="inline-help">
    <p>IAM 장기 키를 등록한 원본 프로필이 필요합니다.</p>
    <button type="button" class="button subtle" data-docs="iam">프로필 설정 문서 ↗</button>
  </div>`;
  return `<fieldset data-auth="iamMfa">
    ${sources.length === 0 ? setup : ""}
    <label class="field">원본 프로필
      <select name="source" required>
        <option value="">IAM 장기 키 프로필 선택</option>
        ${sources.map((profile) => profileOption(profile, source)).join("")}
      </select>
    </label>
    <button class="button subtle registration-link" type="button" data-action="editor-register-iam">IAM 원본 프로필 등록</button>
    <label class="field">갱신 대상 프로필
      <input name="target" list="targets" required maxlength="256" placeholder="예시 work" value="${escape(target)}" />
      <datalist id="targets">${targets.map((profile) => profileOption(profile)).join("")}</datalist>
    </label>
    <p class="field-hint">Terraform과 kubeconfig에서 쓰는 이름으로 설정하세요.</p>
    ${regionField("초기 대상 리전 (선택)", "대상에 리전이 없으면 원본 리전 사용", region ?? "")}
    <label class="field">MFA 장치 ARN 또는 시리얼
      <input name="device" required maxlength="256" spellcheck="false" value="${escape(device)}" />
    </label>
    <div class="mfa-lookup">
      <button type="button" class="button" data-device-lookup aria-describedby="mfa-device-status">MFA 장치 찾기</button>
      <select data-device-options aria-label="조회한 MFA 장치" hidden></select>
      <p id="mfa-device-status" class="muted" data-device-status role="status"></p>
    </div>
    <label class="field">세션 기간 (초, 선택)
      <input name="duration" type="number" min="900" max="129600" step="1" placeholder="AWS CLI 기본값"
        value="${duration ?? ""}" />
    </label>
  </fieldset>`;
}

function ssoFields({ sso, ssoProfile }: Defaults): string {
  return `<fieldset data-auth="sso">
    <label class="field">Identity Center 프로필
      <select name="profile" required>
        <option value="">로컬 SSO 프로필 선택</option>
        ${sso.map((profile) => profileOption(profile, ssoProfile)).join("")}
      </select>
    </label>
    <div class="inline-help">
      <p>새 프로필이 필요하면 조직의 시작 URL 또는 발급자 URL과 SSO 리전을 준비하세요.</p>
      <button class="button subtle" type="button" data-action="editor-wizard">SSO 마법사 열기</button>
    </div>
  </fieldset>`;
}

function editorHtml(items: Profile[], defaults: Defaults, { session, preset }: EditorSelection): string {
  const unsupported = items.filter((profile) => profile.kind === "unsupported");
  return `<form id="session-form">
    <div class="dialog-heading">
      <h2 id="editor-title">${session ? "프로필 편집" : "AWS 프로필 추가"}</h2>
      <button type="button" class="icon-button" data-close="editor" aria-label="닫기">×</button>
    </div>
    <label class="field">인증 방식
      <select name="kind">
        <option value="iamMfa" ${defaults.kind === "iamMfa" ? "selected" : ""}>IAM MFA</option>
        <option value="sso" ${defaults.kind === "sso" ? "selected" : ""}>IAM Identity Center</option>
      </select>
    </label>
    ${iamFields(items, defaults)}
    ${ssoFields(defaults)}
    <label class="field">표시 이름
      <input name="label" placeholder="AWS 프로필 이름 사용" maxlength="64" value="${escape(session?.label ?? preset?.label ?? "")}" />
    </label>
    <label class="field">작업 메모 (선택)
      <textarea name="task" rows="2" maxlength="4000">${escape(preset?.task ?? session?.task ?? "")}</textarea>
    </label>
    ${unsupported.length ? `<details class="unsupported"><summary>지원 범위 외 프로필 (${unsupported.length})</summary>
      <p>${unsupported.map((profile) => escape(profile.name)).join(", ")}</p></details>` : ""}
    <p class="form-error" role="alert" hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="submit">저장</button>
      <button class="button" type="button" data-close="editor">취소</button>
      ${session ? '<button class="button subtle danger" type="button" data-action="remove">목록에서 제거</button>' : ""}
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
