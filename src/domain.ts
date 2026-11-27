export type Page = "sessions" | "cli" | "settings";
export type Login =
  | { kind: "iamMfa"; source: string; target: string; device: string; duration: number | null; region?: string | null }
  | { kind: "sso"; profile: string };
export type Status = "unknown" | "available" | "needsRenewal";
export type Phase = "queued" | "signingIn" | "checking";
export type Language = "en" | "ko";
export type Preferences = { notify: boolean; notifyMinutes: number; language: Language | null };
export type Profile = {
  name: string;
  preferred: boolean;
  kind: "iamSource" | "iamSession" | "sso" | "unconfigured" | "unsupported";
  region: string | null;
  mfaDevice: string | null;
  ssoSession: string | null;
  targetAllowed: boolean;
  inConfig: boolean;
  inCredentials: boolean;
  linkedSource: string | null;
  linkedTarget: string | null;
};
export type Session = {
  id: string;
  label: string;
  task?: string;
  login: Login;
  status: Status;
  phase: Phase | null;
  identity: { Account: string; Arn: string; UserId: string } | null;
  checkedAt: string | null;
  expiresAt: string | null;
  checkMessage: string | null;
  activity: string[] | null;
  canCancel: boolean;
};
export type Dashboard = {
  cli: { path: string; version: string; v2: boolean; mfaSupported: boolean } | null;
  preferredPath: string | null;
  files: { config: string; credentials: string };
  sessions: Session[];
  preferences: Preferences;
  scanning: boolean;
  wizard: boolean;
  profilesError: string | null;
  settingsError: string | null;
};

export function profileName(login: Login): string {
  return login.kind === "sso" ? login.profile : login.target;
}

export function method(login: Login): string {
  return login.kind === "sso" ? "Identity Center" : "IAM MFA";
}

export function selectSession(state: Dashboard | null, selectedId: string | null): Session | undefined {
  const sessions = state?.sessions ?? [];
  return sessions.find((session) => session.id === selectedId) ?? sessions[0];
}

export type SessionDraft = { id: string | null; label: string; login: Login; task?: string };

export type IamRegistration = {
  name: string;
  accessKeyId: string;
  secretAccessKey: string;
  region: string;
};

export type ProfilePreset = { label?: string; task?: string } & (
  | { kind: "iamMfa"; target: string; sourceHint?: string | null; duration?: number | null; region?: string | null }
  | { kind: "sso"; profile: string }
);

export type SharedProfile = {
  version: number;
  profile: string;
  label: string;
  task: string;
  region: string | null;
  authentication:
    | { kind: "iamMfa"; sourceHint: string | null; duration: number | null }
    | { kind: "sso"; settings: {
        startUrl: string; ssoRegion: string; accountId: string; roleName: string;
        session: { name: string; scopes: string | null } | null;
      } };
};

export type ToolConnection = {
  profile: string;
  cli: string;
  config: string;
  credentials: string;
  region: string | null;
  regionConfigured: boolean;
  kind: Profile["kind"];
};

export type ConnectionCheck = {
  status: Status;
  identity: Session["identity"];
  message: string | null;
};
