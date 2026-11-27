import { localizeMessage, t, type MessageKey } from "./i18n";
import type { Dashboard, Profile } from "../domain";
import { escape, fact, header, notice, text } from "./html";

export type InstallerState = "idle" | "downloading" | "opened";

type CliViewOptions = {
  profiles: Profile[] | null;
  cliBusy: boolean;
  installerState: InstallerState;
  latestCliVersion: string | null;
  checkingCliRelease: boolean;
  wizardBusy: boolean;
};

const profileKeys: Record<Exclude<Profile["kind"], "sso">, MessageKey> = {
  iamSource: "cli.iamSource",
  iamSession: "cli.iamSession",
  unconfigured: "cli.unconfigured",
  unsupported: "cli.unsupported",
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
  }
  if (!state.scanning && !state.cli?.v2) {
    messages.push(notice(t("cli.installHelp"), true));
  } else if (!state.scanning && !state.cli?.mfaSupported) {
    messages.push(notice(t("cli.mfaNotDetected"), true));
  }
  if (update) messages.push(notice(t("cli.updateAvailable", { version: update })));
  if (installer === "opened") {
    messages.push(notice(t("cli.installerOpened")));
  }
  return messages.join("");
}

function installerActions(state: Dashboard, options: CliViewOptions, downloadLabel: string): string {
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

function installerCard(state: Dashboard, options: CliViewOptions): string {
  const { installerState, latestCliVersion } = options;
  const cli = state.cli;
  const order = compareVersions(cli?.version, latestCliVersion);
  const update = !state.scanning && order !== null && order < 0 ? latestCliVersion : null;
  const downloadLabel = installerState === "downloading" ? t("cli.downloading")
    : state.scanning ? ""
    : !cli?.v2 ? t("cli.download")
    : update ? t("cli.openUpdate") : "";
  return `<section class="card section-card">
    <div class="card-heading"><div><h2>${cliTitle(state)}</h2>
    <p class="mono path">${escape(cli?.path ?? t("cli.unknownLocation"))}</p></div>
    ${cli?.v2 && !state.scanning ? `<span class="badge available"><span class="dot"></span>${order === 0 ? t("cli.upToDate") : t("cli.connected")}</span>` : ""}</div>
    ${installerNotices(state, installerState, update)}
    ${installerActions(state, options, downloadLabel)}
  </section>`;
}

function profileRow(profile: Profile): string {
  const { name, kind, region, preferred, inConfig, inCredentials } = profile;
  const attributes = `data-profile="${escape(name)}"`;
  const locations = [inConfig && "config", inCredentials && "credentials"].filter(Boolean).join(" · ");
  const usable = kind === "iamSource" || kind === "iamSession" || kind === "sso";
  const attachable = kind === "sso" || profile.targetAllowed;
  return `<li class="profile-row">
    <span class="profile-name"><strong>${escape(name)}</strong>
      <small>${escape(locations || t("cli.unknownFile"))}</small>
      ${region ? `<small>${escape(region)}</small>` : ""}
      ${preferred ? `<small>${text("common.defaultProfile")}</small>` : ""}
    </span>
    <span class="profile-type">${kind === "sso" ? "Identity Center" : text(profileKeys[kind])}</span>
    <div class="profile-actions">
      ${attachable ? `<button class="button subtle" data-workflow="adopt" ${attributes}>${text("cli.linkSession")}</button>` : ""}
      ${usable ? `<button class="button" data-workflow="export" ${attributes}>${text("share.export")}</button>` : ""}
    </div>
  </li>`;
}

function profileList(profiles: Profile[] | null): string {
  if (!profiles?.length) {
    return `<p class="muted">${profiles ? t("cli.noProfiles") : t("cli.loadProfiles")}</p>`;
  }
  return `<ul class="catalog">${profiles.map(profileRow).join("")}</ul>`;
}

function profilesCard(state: Dashboard, { profiles, wizardBusy }: CliViewOptions): string {
  const wizardRunning = wizardBusy || state.wizard;
  const hasCli = state.cli?.v2;
  return `<section class="card section-card">
    <div class="card-heading">
      <h2>${text("cli.localProfiles")}</h2>
      <div class="actions">
        <button class="button" data-action="profiles" ${hasCli ? "" : "disabled"}>${text("cli.reloadProfiles")}</button>
        <button class="button" data-workflow="import">${text("common.importShare")}</button>
      </div>
    </div>
    ${state.profilesError ? notice(localizeMessage(state.profilesError), true) : ""}
    <dl class="facts paths">
      ${fact(t("common.awsConfigFile"), state.files.config, true, true)}
      ${fact(t("common.awsCredentialsFile"), state.files.credentials, true, true)}
    </dl>
    ${profileList(profiles)}
    <div class="wizard-section">
      <h3>${text("cli.registerProfiles")}</h3>
      <p>${text("cli.registrationHelp")}</p>
      <div class="actions">
        <button class="button" data-action="register-iam" ${hasCli ? "" : "disabled"}>${text("cli.registerIam")}</button>
        <button class="button" data-action="wizard" ${!hasCli || wizardRunning ? "disabled" : ""}>
        ${wizardRunning ? t("cli.wizardRunning") : t("common.ssoWizard")}</button>
        <button class="button subtle" data-docs="sso">${text("common.settingsDocs")}</button>
      </div>
    </div>
  </section>`;
}

export function cliPage(state: Dashboard, options: CliViewOptions): string {
  return header("AWS CLI", t("cli.description"))
    + installerCard(state, options) + profilesCard(state, options);
}
