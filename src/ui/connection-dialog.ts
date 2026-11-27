import { localizeMessage, t } from "./i18n";
import type { ConnectionCheck, ToolConnection } from "../domain";
import { element, errorMessage, escape, fact, text } from "./html";
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
    return t("tools.identityVerified", { account: check.identity.Account, arn: check.identity.Arn });
  }
  return check.message ? localizeMessage(check.message, "tools.identityFailed") : t("tools.identityFailed");
}

export function showConnectionDialog(
  dialog: HTMLDialogElement, connection: ToolConnection, check: () => Promise<ConnectionCheck>,
): void {
  dialog.innerHTML = `<div class="dialog-heading"><h2 id="workflow-title">${text("tools.title")}</h2></div>
    <dl class="facts">
      ${fact(t("tools.profileToUse"), connection.profile, true)}
      ${fact(t("common.region"), connection.region ?? t("common.notSet"))}
      ${fact(t("common.awsConfigFile"), connection.config, true, true)}
      ${fact(t("common.awsCredentialsFile"), connection.credentials, true, true)}
    </dl>
    <p class="muted">${text("tools.instructions")}</p>
    ${!connection.regionConfigured && connection.region ? `<p class="notice">${text("tools.inheritedRegion")}</p>` : ""}
    <label class="field">${text("tools.terminal")}
      <select data-shell>
        <option value="posix">macOS · bash / zsh</option>
        <option value="powershell">Windows · PowerShell</option>
      </select>
    </label>
    <details class="tool-eks"><summary>${text("tools.addKubeconfig")}</summary>
      <label class="field">${text("tools.clusterName")}
        <input data-cluster maxlength="100" placeholder="${text("tools.eksClusterName")}" />
      </label>
      <label class="field">${text("tools.eksRegion")}
        <input data-eks-region maxlength="64" value="${escape(connection.region ?? "")}" />
      </label>
    </details>
    <label class="field">${text("tools.commands")}
      <textarea data-commands class="mono" rows="8" readonly></textarea>
    </label>
    <p class="notice" role="status" data-tool-status hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="button" data-copy>${text("tools.copyCommands")}</button>
      <button class="button" type="button" data-test>${text("tools.checkIdentity")}</button>
      <button class="button subtle" type="button" data-close="workflow">${text("common.close")}</button>
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
    status.textContent = t("tools.checkingIdentity");
    test.disabled = true;
    try {
      status.textContent = checkText(await check());
    } catch (error) {
      status.textContent = errorMessage(error, "tools.identityFailed");
    } finally {
      test.disabled = false;
    }
  });
  update();
  if (!dialog.open) dialog.showModal();
}
