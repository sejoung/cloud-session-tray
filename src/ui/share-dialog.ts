import type { SharedProfile } from "../domain";
import { bindSubmit, element, escape, fact } from "./html";
import { shellQuote } from "./shell-commands";
import { copyText } from "./tool-clipboard";

function details(shared: SharedProfile): string {
  const auth = shared.authentication;
  if (auth.kind === "iamMfa") {
    return fact("받는 사람의 원본 프로필 힌트", auth.sourceHint || "직접 선택", true)
      + fact("세션 기간", `${auth.duration ?? 43200}초`)
      + fact("인증 방식", "IAM MFA");
  }
  return fact("인증 방식", "IAM Identity Center") + fact("SSO 리전", auth.settings.ssoRegion)
    + fact("AWS 계정", auth.settings.accountId, true) + fact("역할", auth.settings.roleName, true)
    + fact("시작 URL 또는 발급자 URL", auth.settings.startUrl, true, true);
}

function profileGuide(shared: SharedProfile): string {
  const lines = [shared.label, `AWS 프로필 ${shared.profile}`];
  if (shared.region) lines.push(`리전 ${shared.region}`);
  const auth = shared.authentication;
  if (auth.kind === "iamMfa") {
    lines.push(
      `원본 프로필 ${auth.sourceHint || "본인의 장기 IAM 키가 있는 로컬 프로필"}`,
      `갱신 대상 ${shared.profile}`,
      `세션 기간 ${auth.duration ?? 43200}초`,
      "CST 또는 AWS CLI v2에서 본인의 MFA 장치로 갱신하세요.",
      `aws configure mfa-login --profile=${shellQuote(auth.sourceHint || "default", "posix")} --update-profile=${shellQuote(shared.profile, "posix")} --duration-seconds=${auth.duration ?? 43200}`,
      `Terraform과 AWS CLI 등에서 ${shared.profile} 프로필을 지정하세요.`,
    );
  } else {
    const { startUrl, ssoRegion, accountId, roleName } = auth.settings;
    lines.push(
      `Identity Center ${startUrl}`, `SSO 리전 ${ssoRegion}`, `계정 ${accountId} · 역할 ${roleName}`,
      `AWS CLI 또는 CST에서 ${shared.profile} 프로필을 만들고 본인의 계정으로 로그인하세요.`,
    );
  }
  if (shared.task.trim()) lines.push("", "작업 메모", shared.task.trim());
  return lines.join("\n");
}

function profileInput(shared: SharedProfile): string {
  const label = shared.authentication.kind === "iamMfa" ? "갱신 대상 프로필" : "AWS 프로필 이름";
  return `<label class="field">${label}
    <input name="profile" required maxlength="256" value="${escape(shared.profile)}" />
  </label>`;
}

export function showShareDialog(
  dialog: HTMLDialogElement, shared: SharedProfile, save: (value: SharedProfile) => Promise<boolean>,
): void {
  dialog.innerHTML = `<form>
    <div class="dialog-heading"><h2 id="workflow-title">AWS 프로필 공유</h2></div>
    <p class="dialog-summary">받는 사람이 자신의 AWS 자격 증명으로 같은 프로필을 설정할 수 있도록 안내를 공유합니다.</p>
    <dl class="facts">${details(shared)}${fact("리전", shared.region ?? "미설정")}</dl>
    ${profileInput(shared)}
    <label class="field">작업 메모 (선택)
      <textarea name="task" rows="3" maxlength="4000">${escape(shared.task)}</textarea>
    </label>
    <label class="field">팀에 전달할 안내
      <textarea data-guide readonly rows="6"></textarea>
    </label>
    <p class="muted" role="status" data-copy-status></p>
    <p class="form-error" role="alert" hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="button" data-copy>안내 복사</button>
      <button class="button" type="submit">공유 파일 내보내기</button>
      <button class="button subtle" type="button" data-close="workflow">닫기</button>
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
    if (await save(preview())) status.textContent = "공유 파일을 저장했습니다.";
  });
  refresh();
  if (!dialog.open) dialog.showModal();
}

export function showImportDialog(
  dialog: HTMLDialogElement, shared: SharedProfile, apply: (value: SharedProfile) => Promise<void>,
): void {
  dialog.innerHTML = `<form>
    <div class="dialog-heading"><h2 id="workflow-title">AWS 공유 프로필 가져오기</h2></div>
    <p class="dialog-summary">${escape(shared.label)}</p>
    ${profileInput(shared)}
    <dl class="facts">${details(shared)}${fact("리전", shared.region ?? "미설정")}</dl>
    ${shared.task ? `<section class="task-note"><h3>작업 메모</h3><p>${escape(shared.task)}</p></section>` : ""}
    <p class="form-error" role="alert" hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="submit">${shared.authentication.kind === "sso" ? "프로필 가져오기" : "IAM 설정 열기"}</button>
      <button class="button" type="button" data-close="workflow">취소</button>
    </div>
  </form>`;
  const form = element<HTMLFormElement>(dialog, "form");
  bindSubmit(form, async () => {
    const profile = element<HTMLInputElement>(form, '[name="profile"]').value.trim();
    await apply({ ...shared, profile });
    if (dialog.open) dialog.close();
  });
}
