import { vi } from "vitest";
import type { DesktopApi } from "../src/desktop-api";
import type { Dashboard, Login, Profile, Session } from "../src/domain";

export const cli: NonNullable<Dashboard["cli"]> = {
  path: "/local/bin/aws", version: "2.40.4", v2: true, mfaSupported: true,
};

export function profile(name: string, kind: Profile["kind"], fields: Partial<Profile> = {}): Profile {
  return {
    name, kind, preferred: false, region: "ap-northeast-2",
    mfaDevice: null, ssoSession: null, inConfig: true, inCredentials: false, linkedSource: null, linkedTarget: null, targetAllowed: kind === "iamSession", ...fields,
  };
}

export function mockSession(id: string, label: string, login: Login): Session {
  return {
    id, label, login, status: "unknown", phase: null, identity: null,
    checkedAt: null, expiresAt: null, checkMessage: null, activity: null, canCancel: false,
  };
}

export function backend(state: Partial<Dashboard> = {}, profiles: Profile[] = []) {
  const dashboard: Dashboard = {
    cli, cliInstallations: [], preferredPath: null, files: { config: "/test/config", credentials: "/test/credentials" },
    terminalPath: { supported: false, enabled: false, error: null },
    sessions: [], preferences: { notify: false, notifyMinutes: 10, language: "ko" },
    scanning: false, wizard: false, profilesError: null, settingsError: null, ...state,
  };
  const listeners = new Map<string, (value: string) => void>();
  const api = {
    dashboard: vi.fn<DesktopApi["dashboard"]>(async () => structuredClone(dashboard)),
    profiles: vi.fn<DesktopApi["profiles"]>(async () => profiles),
    prepareShare: vi.fn<DesktopApi["prepareShare"]>(async (name, task) => ({
      version: 1, profile: name, label: name, task, region: "ap-northeast-2",
      authentication: { kind: "iamMfa", sourceHint: "default", duration: 43200 },
    })),
    saveShare: vi.fn<DesktopApi["saveShare"]>(async () => true),
    openShare: vi.fn<DesktopApi["openShare"]>(async () => null),
    importSso: vi.fn<DesktopApi["importSso"]>(async () => "saved"),
    mfaDevices: vi.fn<DesktopApi["mfaDevices"]>(async () => []),
    registerIam: vi.fn<DesktopApi["registerIam"]>(async () => profiles),
    configureSso: vi.fn<DesktopApi["configureSso"]>(async () => profiles),
    cli: vi.fn<DesktopApi["cli"]>(async () => undefined),
    session: vi.fn<DesktopApi["session"]>(async () => {}),
    saveSession: vi.fn<DesktopApi["saveSession"]>(async (draft) => {
      const id = draft.id ?? "saved";
      dashboard.sessions = [{ ...mockSession(id, draft.label, draft.login), task: draft.task }];
      return id;
    }),
    renewMfa: vi.fn<DesktopApi["renewMfa"]>(async () => {}),
    savePreferences: vi.fn<DesktopApi["savePreferences"]>(async (preferences) => {
      dashboard.preferences = preferences;
    }),
    setSystemLanguage: vi.fn<DesktopApi["setSystemLanguage"]>(async () => {}),
    openLogFolder: vi.fn<DesktopApi["openLogFolder"]>(async () => {}),
    latestCliVersion: vi.fn<DesktopApi["latestCliVersion"]>(async () => cli.version),
    installCli: vi.fn<DesktopApi["installCli"]>(async () => {}),
    openDocs: vi.fn<DesktopApi["openDocs"]>(async () => {}),
    allowNotifications: vi.fn<DesktopApi["allowNotifications"]>(async () => true),
    onTrayEvent: vi.fn<DesktopApi["onTrayEvent"]>(async (event, handler) => {
      listeners.set(event, handler);
      return () => listeners.delete(event);
    }),
  };
  return { api, dashboard, emit: (event: string, value: string) => listeners.get(event)?.(value) };
}
