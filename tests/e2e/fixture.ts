import { mountApplication } from "../../src/app";
import type { DesktopApi } from "../../src/desktop-api";
import { profileName, type Dashboard, type Profile, type Session, type SharedProfile } from "../../src/domain";
import { element } from "../../src/ui/html";
import "../../src/style.css";

const scenario = new URLSearchParams(location.search).get("scenario");
const sso = scenario === "sso";
const layout = scenario === "layout";
const duplicate = scenario === "duplicate-cli";
const cli = { path: "/usr/local/bin/aws", version: "2.42.4", v2: true, mfaSupported: true };
const olderCli = { ...cli, path: "/opt/homebrew/bin/aws", version: "2.40.4" };
const device = "arn:aws:iam::123456789012:mfa/example";
const dashboard: Dashboard = {
  cli: sso || layout || duplicate ? cli : null,
  cliInstallations: duplicate ? [
    { ...cli, source: "awsInstaller", canRemove: false },
    { ...olderCli, source: "homebrew", canRemove: true },
  ] : [],
  terminalPath: { supported: duplicate, enabled: false, error: null },
  preferredPath: null,
  files: { config: "~/.aws/config", credentials: "~/.aws/credentials" },
  sessions: [],
  preferences: { notify: true, notifyMinutes: 10, language: null },
  scanning: false,
  wizard: false,
  profilesError: null,
  settingsError: null,
};

function profile(name: string, kind: "sso" | "iamSource", region: string | null): Profile {
  return {
    name, kind, region, preferred: true, mfaDevice: null, targetAllowed: false,
    ssoSession: kind === "sso" ? "team" : null, inConfig: true, inCredentials: kind === "iamSource",
    linkedSource: null, linkedTarget: null,
  };
}

const ssoProfile = profile("team-dev", "sso", "ap-northeast-2");
const profiles = sso ? [ssoProfile] : [];
if (layout) {
  const path = `/example/${"long-workspace-name".repeat(8)}/.aws`;
  dashboard.files = { config: `${path}/config`, credentials: `${path}/credentials` };
  profiles.push(profile("work-source", "iamSource", "us-gov-west-1"), ssoProfile);
  const examples: Pick<Session, "id" | "login" | "label">[] = [
    { id: "example-iam", label: "Development", login: {
      kind: "iamMfa", source: "work-source", target: `dev-${"long-profile-name".repeat(12)}`,
      region: "us-gov-west-1", device, duration: 43200,
    } },
    { id: "example-sso", label: "Development SSO", login: { kind: "sso", profile: ssoProfile.name } },
  ];
  dashboard.sessions = examples.map((example) => ({
    ...example, status: "unknown", task: "Review the development deployment.\n".repeat(80),
    phase: null, canCancel: false, checkedAt: null, identity: null, expiresAt: null, checkMessage: null, activity: null,
  }));
}
let cliInstalled = Boolean(dashboard.cli);
let shareFile: string | undefined;
const unavailable = async () => { throw new Error("Unexpected desktop action in browser test"); };

function markAvailable(id: string): void {
  const session = dashboard.sessions.find((item) => item.id === id);
  if (!session) throw new Error(`Missing test session ${id}`);
  session.status = "available";
  session.checkedAt = "2026-11-27T01:04:00Z";
  session.identity = { Account: "123456789012", Arn: "arn:aws:iam::123456789012:user/example", UserId: "EXAMPLE" };
  if (session.login.kind === "iamMfa") session.expiresAt = "2026-11-27T13:04:00Z";
}

const api: DesktopApi = {
  dashboard: async () => structuredClone(dashboard),
  profiles: async () => profiles,
  latestCliVersion: async () => cli.version,
  installCli: async () => { cliInstalled = true; },
  cli: async (command, path) => {
    if (command === "rescan_cli" && cliInstalled) dashboard.cli = cli;
    if (command === "enable_terminal_path") dashboard.terminalPath.enabled = true;
    if (command === "disable_terminal_path") dashboard.terminalPath.enabled = false;
    if (duplicate && command === "choose_cli" && path === olderCli.path) {
      dashboard.cli = olderCli;
      dashboard.preferredPath = path;
      return true;
    }
    if (duplicate && command === "reset_cli") {
      dashboard.cli = cli;
      dashboard.preferredPath = null;
    }
    if (duplicate && command === "remove_cli" && path === olderCli.path) {
      if (!window.confirm("Remove the example Homebrew installation?")) return false;
      dashboard.cliInstallations = dashboard.cliInstallations.filter((item) => item.path !== path);
      return true;
    }
  },
  registerIam: async ({ name, region }) => {
    profiles.push(profile(name, "iamSource", region || null));
    return profiles;
  },
  mfaDevices: async () => [device],
  configureSso: async () => profiles,
  saveSession: async ({ login, label, task }) => {
    const id = profileName(login);
    dashboard.sessions.push({
      id, login, label, task, status: "unknown", phase: null, canCancel: false,
      identity: null, checkedAt: null, expiresAt: null, checkMessage: null, activity: null,
    });
    return id;
  },
  renewMfa: async (id, otp) => {
    if (otp !== "123456") throw new Error("Incorrect example OTP");
    markAvailable(id);
  },
  session: async (command, id) => {
    if (command === "check_session" || command === "renew_sso") markAvailable(id);
  },
  onTrayEvent: async () => () => {},
  allowNotifications: async () => true,
  savePreferences: async (preferences) => { dashboard.preferences = preferences; },
  setSystemLanguage: unavailable,
  openLogFolder: unavailable,
  openDocs: unavailable,
  prepareShare: async (name, task) => {
    const session = dashboard.sessions.find(({ login }) => profileName(login) === name);
    if (!session) throw new Error(`No example session for ${name}`);
    const summary = { version: 1, profile: name, label: session.label, task };
    if (session.login.kind === "sso") {
      return { ...summary, region: ssoProfile.region, authentication: { kind: "sso", settings: {
        startUrl: `https://identity.awsapps.com/start/${"example".repeat(28)}`,
        ssoRegion: "us-east-1", accountId: "123456789012", roleName: "ReadOnly", session: null,
      } } };
    }
    const { source, duration, region } = session.login;
    const sourceRegion = profiles.find((item) => item.name === source)?.region;
    return {
      ...summary, region: region ?? sourceRegion ?? null,
      authentication: { kind: "iamMfa", sourceHint: source, duration },
    };
  },
  saveShare: async (shared) => {
    shareFile = JSON.stringify(shared);
    return true;
  },
  openShare: async () => shareFile ? JSON.parse(shareFile) as SharedProfile : null,
  importSso: unavailable,
};

mountApplication(element<HTMLElement>(document, "#app"), api);
