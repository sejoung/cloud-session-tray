import type { ConnectionCheck, ToolConnection } from "../domain";
import { element, errorMessage, escape, fact } from "./html";
import { copyText } from "./tool-clipboard";

import { shellQuote, type Shell } from "./shell-commands";

const conflicting = [
  "AWS_DEFAULT_PROFILE", "AWS_ACCESS_KEY_ID", "AWS_ACCESS_KEY", "AWS_SECRET_ACCESS_KEY",
  "AWS_SECRET_KEY", "AWS_SESSION_TOKEN", "AWS_SECURITY_TOKEN", "AWS_CREDENTIAL_FILE",
  "AWS_ROLE_ARN", "AWS_ROLE_SESSION_NAME", "AWS_WEB_IDENTITY_TOKEN_FILE",
  "AWS_CONTAINER_CREDENTIALS_RELATIVE_URI", "AWS_CONTAINER_CREDENTIALS_FULL_URI",
  "AWS_CONTAINER_AUTHORIZATION_TOKEN", "AWS_CONTAINER_AUTHORIZATION_TOKEN_FILE",
];

function connectionCommands(connection: ToolConnection, shell: Shell, cluster: string, eksRegion: string): string {
  const quote = (value: string) => shellQuote(value, shell);
  const aws = `${shell === "powershell" ? "& " : ""}${quote(connection.cli)}`;
  const values: Record<string, string> = {
    AWS_PROFILE: connection.profile, AWS_CONFIG_FILE: connection.config,
    AWS_SHARED_CREDENTIALS_FILE: connection.credentials, AWS_SDK_LOAD_CONFIG: "1",
  };
  if (connection.region) {
    values.AWS_REGION = connection.region;
    values.AWS_DEFAULT_REGION = connection.region;
  }
  const reset = shell === "powershell"
    ? `Remove-Item ${conflicting.map((name) => `Env:${name}`).join(", ")} -ErrorAction SilentlyContinue`
    : `unset ${conflicting.join(" ")}`;
  const lines = [reset, ...Object.entries(values).map(([key, value]) =>
    shell === "powershell" ? `$Env:${key} = ${quote(value)}` : `export ${key}=${quote(value)}`)];
  lines.push(`${aws} sts get-caller-identity --profile=${quote(connection.profile)}`);
  if (validCluster(cluster) && validRegion(eksRegion)) {
    lines.push(`${aws} eks update-kubeconfig --name=${quote(cluster)} --region=${quote(eksRegion)} --profile=${quote(connection.profile)}`);
  }
  return lines.join("\n");
}

function validCluster(value: string): boolean {
  return /^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$/.test(value);
}

function validRegion(value: string): boolean {
  return /^[A-Za-z0-9-]{1,64}$/.test(value);
}

function checkText(check: ConnectionCheck): string {
  if (check.status === "available" && check.identity) {
    return `AWS 신원 확인 완료 · ${check.identity.Account} · ${check.identity.Arn}`;
  }
  return check.message ?? "AWS 신원을 확인할 수 없습니다.";
}

export function showConnectionDialog(
  dialog: HTMLDialogElement, connection: ToolConnection, check: () => Promise<ConnectionCheck>,
): void {
  dialog.innerHTML = `<div class="dialog-heading"><h2 id="workflow-title">CLI · 외부 도구 연결</h2></div>
    <dl class="facts">
      ${fact("도구에 지정할 AWS 프로필", connection.profile, true)}
      ${fact("리전", connection.region ?? "미설정")}
      ${fact("AWS 설정 파일", connection.config, true, true)}
      ${fact("AWS 자격 증명 파일", connection.credentials, true, true)}
    </dl>
    <p class="muted">터미널에서 아래 설정을 적용한 뒤 Terraform 같은 도구를 같은 터미널에서 실행하세요.</p>
    ${!connection.regionConfigured && connection.region ? '<p class="notice">대상 프로필에 리전이 비어 있어 원본 설정의 리전을 사용합니다.</p>' : ""}
    <label class="field">터미널
      <select data-shell>
        <option value="posix">macOS · bash / zsh</option>
        <option value="powershell">Windows · PowerShell</option>
      </select>
    </label>
    <details class="tool-eks"><summary>EKS kubeconfig 명령 추가</summary>
      <label class="field">클러스터 이름
        <input data-cluster maxlength="100" placeholder="EKS 클러스터 이름" />
      </label>
      <label class="field">EKS 리전
        <input data-eks-region maxlength="64" value="${escape(connection.region ?? "")}" />
      </label>
    </details>
    <label class="field">터미널 명령
      <textarea data-commands class="mono" rows="8" readonly></textarea>
    </label>
    <p class="notice" role="status" data-tool-status hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="button" data-copy>명령 복사</button>
      <button class="button" type="button" data-test>지금 신원 확인</button>
      <button class="button subtle" type="button" data-close="workflow">닫기</button>
    </div>`;
  const shell = element<HTMLSelectElement>(dialog, "[data-shell]");
  const commands = element<HTMLTextAreaElement>(dialog, "[data-commands]");
  const cluster = element<HTMLInputElement>(dialog, "[data-cluster]");
  const region = element<HTMLInputElement>(dialog, "[data-eks-region]");
  const status = element<HTMLElement>(dialog, "[data-tool-status]");
  const test = element<HTMLButtonElement>(dialog, "[data-test]");
  const update = () => {
    commands.value = connectionCommands(connection, shell.value as Shell, cluster.value.trim(), region.value.trim());
  };
  shell.value = navigator.platform.startsWith("Win") ? "powershell" : "posix";
  for (const input of [shell, cluster, region]) input.addEventListener("input", update);
  element(dialog, "[data-copy]").addEventListener("click", () => {
    status.hidden = false;
    void copyText(commands, status);
  });
  test.addEventListener("click", async () => {
    status.hidden = false;
    status.textContent = "AWS 신원을 확인하는 중…";
    test.disabled = true;
    try {
      status.textContent = checkText(await check());
    } catch (error) {
      status.textContent = errorMessage(error, "AWS 신원을 확인할 수 없습니다.");
    } finally {
      test.disabled = false;
    }
  });
  update();
  if (!dialog.open) dialog.showModal();
}
