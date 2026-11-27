import { mountApplication } from "../../src/app";
import type { DesktopApi } from "../../src/desktop-api";
import { profileName, type Dashboard, type Profile, type SharedProfile } from "../../src/domain";
import { element } from "../../src/ui/html";
import "../../src/style.css";

const sso = new URLSearchParams(location.search).get("scenario") === "sso";
const cli = { path: "/usr/local/bin/aws", version: "2.42.4", v2: true, mfaSupported: true };
const device = "arn:aws:iam::123456789012:mfa/example";
const dashboard: Dashboard = {
  cli: sso ? cli : null,
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
let cliInstalled = sso;
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
  cli: async (command) => {
    if (command === "rescan_cli" && cliInstalled) dashboard.cli = cli;
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
  toolConnection: unavailable,
  testConnection: unavailable,
  prepareShare: async (name, task) => {
    const session = dashboard.sessions.find(({ login }) => login.kind === "iamMfa" && login.target === name);
    if (!session || session.login.kind !== "iamMfa") throw new Error(`No example IAM session for ${name}`);
    const { source, duration, region } = session.login;
    const sourceRegion = profiles.find((item) => item.name === source)?.region;
    return {
      version: 1, profile: name, label: session.label, task, region: region ?? sourceRegion ?? null,
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
