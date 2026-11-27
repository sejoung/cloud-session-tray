import { localizeMessage, t, type MessageKey } from "../../shared/i18n";
import type { Dashboard, Profile } from "../../shared/domain";
import { escape, fact, notice, text } from "../../shared/html";

export type LocalProfilesOptions = {
  profiles: Profile[] | null;
  wizardBusy: boolean;
};

const profileKeys: Record<Exclude<Profile["kind"], "sso">, MessageKey> = {
  iamSource: "cli.iamSource",
  iamSession: "cli.iamSession",
  unconfigured: "cli.unconfigured",
  unsupported: "cli.unsupported",
};

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

export function localProfilesCard(state: Dashboard, { profiles, wizardBusy }: LocalProfilesOptions): string {
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
