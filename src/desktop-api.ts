import type { Dashboard, IamRegistration, Language, Preferences, Profile, SessionDraft, SharedProfile } from "./domain";

type CliCommand = "rescan_cli" | "choose_cli" | "reset_cli";
type SessionCommand = "renew_sso" | "check_session" | "cancel_session" | "remove_session";
type TrayEvent = "select-session" | "request-otp" | "session-error";

export interface DesktopApi {
  dashboard(): Promise<Dashboard>;
  profiles(): Promise<Profile[]>;
  prepareShare(profile: string, task: string): Promise<SharedProfile>;
  saveShare(shared: SharedProfile): Promise<boolean>;
  openShare(): Promise<SharedProfile | null>;
  importSso(shared: SharedProfile): Promise<string>;
  mfaDevices(source: string): Promise<string[]>;
  registerIam(registration: IamRegistration): Promise<Profile[]>;
  configureSso(): Promise<Profile[]>;
  cli(command: CliCommand): Promise<boolean | void>;
  session(command: SessionCommand, id: string): Promise<void>;
  saveSession(draft: SessionDraft): Promise<string>;
  renewMfa(id: string, otp: string): Promise<void>;
  savePreferences(preferences: Preferences): Promise<void>;
  setSystemLanguage(language: Language): Promise<void>;
  openLogFolder(): Promise<void>;
  latestCliVersion(force: boolean): Promise<string | null>;
  installCli(): Promise<void>;
  openDocs(topic: string): Promise<void>;
  allowNotifications(): Promise<boolean>;
  onTrayEvent(name: TrayEvent, handle: (message: string) => void): Promise<() => void>;
}
