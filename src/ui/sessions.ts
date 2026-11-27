import { localizeMessage, t, type MessageKey } from "./i18n";
import appIcon from "../../src-tauri/icons/icon.png";
import { method, profileName, type Dashboard, type Phase, type Profile, type Session, type Status } from "../domain";
import { date, escape, fact, header, notice, text } from "./html";

const statusKeys: Record<Status, MessageKey> = {
  unknown: "common.notChecked",
  available: "sessions.available",
  needsRenewal: "sessions.needsRenewal",
};
const phaseKeys: Record<Phase, MessageKey> = {
  queued: "sessions.queued",
  signingIn: "sessions.signingIn",
  checking: "sessions.checking",
};

function badge(session: Session): string {
  const label = text(session.phase ? phaseKeys[session.phase] : statusKeys[session.status]);
  const appearance = session.phase ? "working" : session.status;
  return `<span class="badge ${appearance}"><span class="dot"></span>${label}</span>`;
}

function onboarding(hasCli: boolean, profiles: Profile[] | null): string {
  const hasIam = profiles?.some((profile) => profile.kind === "iamSource");
  const hasSso = profiles?.some((profile) => profile.kind === "sso");
  const firstRegistration = hasIam === false && !hasSso;
  const heading = hasCli ? t("sessions.firstProfile") : t("sessions.connectCli");
  const description = !hasCli ? t("sessions.connectCliHelp")
    : firstRegistration ? t("sessions.registerFirst")
    : t("sessions.connectExisting");
  const actions = hasCli ? `
    ${hasIam === false ? `<button class="button ${firstRegistration ? "primary" : ""}" data-action="start-iam">${text("sessions.registerKeys")}</button>` : ""}
    <button class="button ${firstRegistration ? "" : "primary"}" data-action="add">
      ${firstRegistration ? t("sessions.chooseAuthentication") : t("sessions.connectProfile")}
    </button>` : `<button class="button primary" data-page="cli">${text("sessions.openCli")}</button>`;
  return `<section class="onboarding">
    <img src="${appIcon}" alt="" width="58" height="58" />
    <h2>${heading}</h2>
    <p>${description}</p>
    <div class="actions">${actions}</div>
  </section>`;
}

function sessionList(sessions: Session[], selected: Session): string {
  const rows = sessions.map((item) => `<li><button class="session-row" data-session="${escape(item.id)}"
    aria-pressed="${item.id === selected.id}">
    <span class="row-label"><strong>${escape(item.label)}</strong><span>${escape(profileName(item.login))}</span></span>
    ${badge(item)}
  </button></li>`).join("");
  return `<section class="card session-list" aria-label="${text("sessions.savedProfiles")}">
    <h2>${text("common.profile")} <span>${sessions.length}</span></h2><ul>${rows}</ul>
  </section>`;
}

function sessionFacts(session: Session): string {
  const { login, identity } = session;
  const expiry = login.kind === "sso" ? t("sessions.ssoExpiry") : date(session.expiresAt);
  return `<dl class="facts">
    ${fact(t("common.awsProfile"), profileName(login), true)}
    ${login.kind === "iamMfa" ? fact(t("common.sourceProfile"), login.source, true) : ""}
    ${fact(t("common.account"), identity?.Account ?? t("common.notChecked"), true)}
    ${fact(t("sessions.lastCheck"), session.checkedAt ? date(session.checkedAt) : t("common.notChecked"))}
    ${fact(t("sessions.expiration"), expiry)}
    ${fact(t("sessions.identityArn"), identity?.Arn ?? t("common.notChecked"), true, true)}
  </dl>`;
}

function sessionActions(session: Session, hasCli: boolean, requiresMfa: boolean): string {
  const canStart = hasCli && !session.phase;
  return `<div class="actions detail-actions">
    <button class="button primary" data-action="renew" ${canStart && !requiresMfa ? "" : "disabled"}>${text("common.renew")}</button>
    <button class="button" data-action="check" ${canStart ? "" : "disabled"}>${text("common.checkStatus")}</button>
    <button class="button subtle" data-workflow="connect" ${hasCli ? "" : "disabled"}>${text("common.connectTools")}</button>
    <button class="button subtle" data-workflow="share">${text("common.share")}</button>
    ${session.phase ? `<button class="button subtle" data-action="cancel" ${!session.canCancel ? "disabled" : ""}>
      ${session.canCancel ? t("common.cancel") : t("common.canceling")}</button>`
      : `<button class="button subtle" data-action="edit" ${hasCli ? "" : "disabled"}>${text("common.edit")}</button>`}
  </div>`;
}

function sessionDetail(session: Session, cli: Dashboard["cli"]): string {
  const hasCli = Boolean(cli?.v2);
  const requiresMfa = session.login.kind === "iamMfa" && !cli?.mfaSupported;
  return `<section class="card detail" aria-label="${text("sessions.selected")}">
    <div class="detail-top">
      <div><p class="method">${method(session.login)}</p><h2>${escape(session.label)}</h2></div>
      ${badge(session)}
    </div>
    ${sessionFacts(session)}
    ${session.task ? `<section class="task-note"><h3>${text("common.task")}</h3><p>${escape(session.task)}</p></section>` : ""}
    ${session.checkMessage ? notice(localizeMessage(session.checkMessage)) : ""}
    ${session.activity ? notice(session.activity.map((message) => localizeMessage(message)).join(" ")) : ""}
    ${requiresMfa && hasCli ? notice(t("sessions.mfaUpdateHelp"), true) : ""}
    ${sessionActions(session, hasCli, requiresMfa)}
  </section>`;
}

export function sessionsPage(state: Dashboard, profiles: Profile[] | null, session?: Session): string {
  const hasCli = Boolean(state.cli?.v2);
  const add = session
    ? `<button class="button primary" data-action="add" ${hasCli ? "" : "disabled"}>${text("common.addProfile")}</button>` : "";
  const banners = [state.settingsError, state.profilesError]
    .filter((message): message is string => Boolean(message))
    .map((message) => notice(localizeMessage(message), true)).join("");
  const body = session
    ? `<div class="session-grid">${sessionList(state.sessions, session)}${sessionDetail(session, state.cli)}</div>`
    : onboarding(hasCli, profiles);
  const actions = `<div class="actions">${add}<button class="button" data-workflow="import">${text("common.importShare")}</button></div>`;
  return header(t("sessions.title"), t("sessions.description"), actions) + banners + body;
}
