import { t } from "../../shared/i18n";
import type { Dashboard } from "../../shared/domain";
import { escape, notice, text } from "../../shared/html";
import { activeInstallation, installationSources } from "./installations";

export type InstallerState = "idle" | "downloading" | "opened";
export type InstallerOptions = {
  cliBusy: boolean;
  installerState: InstallerState;
  latestCliVersion: string | null;
  checkingCliRelease: boolean;
};

function cliTitle(state: Dashboard): string {
  if (state.scanning) return t("cli.scanning");
  if (!state.cli) return t("cli.notFound");
  const version = escape(state.cli.version);
  return state.cli.v2 ? `AWS CLI v${version}` : text("cli.versionRequiresUpdate", { version: state.cli.version });
}

function compareVersions(installed: string | undefined, published: string | null): number | null {
  const pattern = /^2\.(\d+)\.(\d+)$/;
  const current = installed?.match(pattern);
  const latest = published?.match(pattern);
  if (!current || !latest) return null;
  return Number(current[1]) - Number(latest[1]) || Number(current[2]) - Number(latest[2]);
}

function installerNotices(state: Dashboard, installer: InstallerState, update: string | null): string {
  const messages: string[] = [];
  if (state.preferredPath && state.cli?.path !== state.preferredPath) {
    messages.push(notice(t("cli.fallbackPath"), true));
  } else if (state.preferredPath && update) {
    messages.push(notice(t("cli.selectedPathUpdate")));
  }
  if (!state.scanning) {
    if (!state.cli?.v2) messages.push(notice(t("cli.installHelp"), true));
    else if (!state.cli.mfaSupported) messages.push(notice(t("cli.mfaNotDetected"), true));
  }
  if (update) {
    messages.push(notice(t("cli.updateAvailable", { version: update })));
    const source = activeInstallation(state)?.source;
    if (source === "homebrew" || source === "awsUser") messages.push(notice(t("cli.separateInstaller"), true));
  }
  if (installer === "opened") {
    messages.push(notice(t("cli.installerOpened")));
  }
  return messages.join("");
}

function installerActions(state: Dashboard, options: InstallerOptions, downloadLabel: string): string {
  const { cliBusy, installerState, checkingCliRelease } = options;
  return `<div class="actions">
      ${downloadLabel ? `<button class="button primary" data-action="install" ${installerState === "downloading" ? "disabled" : ""}>${downloadLabel}</button>` : ""}
      <button class="button" data-action="rescan" ${cliBusy || state.scanning ? "disabled" : ""}>${text("cli.rescan")}</button>
      <button class="button" data-action="choose" ${cliBusy ? "disabled" : ""}>${text("cli.selectExecutable")}</button>
      ${state.preferredPath ? `<button class="button subtle" data-action="auto-cli">${text("cli.autoDiscover")}</button>` : ""}
      ${state.cli?.v2 ? `<button class="button subtle" data-action="check-cli-release" ${checkingCliRelease || state.scanning ? "disabled" : ""}>
        ${checkingCliRelease ? t("cli.checkingUpdates") : t("cli.checkUpdates")}</button>` : ""}
      <button class="button subtle" data-docs="installation">${text("cli.installDocs")}</button>
    </div>`;
}

function installerHeading(state: Dashboard, upToDate: boolean): string {
  const cli = state.cli;
  const active = activeInstallation(state);
  return `<div class="card-heading"><div><h2>${cliTitle(state)}</h2>
    <p class="mono path">${escape(cli?.path ?? t("cli.unknownLocation"))}</p>
    ${active ? `<p class="installation-path">${text(installationSources[active.source])}</p>` : ""}</div>
    ${cli?.v2 && !state.scanning ? `<span class="badge available"><span class="dot"></span>${upToDate ? t("cli.upToDate") : t("cli.connected")}</span>` : ""}</div>`;
}

export function installerCard(state: Dashboard, options: InstallerOptions): string {
  const { installerState, latestCliVersion } = options;
  const cli = state.cli;
  const order = compareVersions(cli?.version, latestCliVersion);
  const update = !state.scanning && order !== null && order < 0 ? latestCliVersion : null;
  const downloadLabel = installerState === "downloading" ? t("cli.downloading")
    : state.scanning ? ""
    : !cli?.v2 ? t("cli.download")
    : update ? t("cli.openUpdate") : "";
  return `<section class="card section-card">
    ${installerHeading(state, order === 0)}
    ${installerNotices(state, installerState, update)}
    ${installerActions(state, options, downloadLabel)}
  </section>`;
}
