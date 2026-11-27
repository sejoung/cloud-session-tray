import type { MessageKey } from "../../shared/i18n";
import type { CliInstallation, Dashboard } from "../../shared/domain";
import { escape, text } from "../../shared/html";

export const installationSources: Record<CliInstallation["source"], MessageKey> = {
  homebrew: "cli.sourceHomebrew",
  awsUser: "cli.sourceAwsUser",
  awsInstaller: "cli.sourceAwsInstaller",
  unknown: "cli.sourceUnknown",
};

export function activeInstallation(state: Dashboard): CliInstallation | undefined {
  return state.cliInstallations.find((item) => item.path === state.cli?.path);
}

function installationRow(item: CliInstallation, active: boolean, busy: boolean): string {
  const path = escape(item.path);
  const disabled = busy ? "disabled" : "";
  return `<li class="profile-row">
    <span class="profile-name">
      <strong>AWS CLI v${escape(item.version)}</strong> · ${text(installationSources[item.source])}
      <span class="mono installation-path">${path}</span>
    </span>
    ${active ? `<span class="badge available"><span class="dot"></span>${text("cli.installationInUse")}</span>`
      : `<div class="profile-actions">
        ${item.v2 ? `<button class="button" data-action="choose" data-path="${path}" ${disabled}>${text("cli.useInstallation")}</button>` : ""}
        ${item.canRemove ? `<button class="button danger" data-action="remove-cli" data-path="${path}" ${disabled}>${text("common.remove")}</button>`
          : item.source === "awsInstaller" ? `<button class="button" data-docs="uninstallation">${text("cli.uninstallDocs")}</button>` : ""}
      </div>`}
  </li>`;
}

export function installationsCard(state: Dashboard, busy: boolean): string {
  if (state.cliInstallations.length < 2) return "";
  return `<section class="card section-card" aria-labelledby="cli-installations">
    <h2 id="cli-installations">${text("cli.installations", { count: state.cliInstallations.length })}</h2>
    <ul class="catalog cli-installations">${state.cliInstallations
      .map((item) => installationRow(item, item.path === state.cli?.path, busy || state.scanning)).join("")}</ul>
  </section>`;
}
