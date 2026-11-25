import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { isPermissionGranted, requestPermission } from "@tauri-apps/plugin-notification";
import type { DesktopApi } from "./desktop-api";

export const desktop: DesktopApi = {
  dashboard: () => invoke("dashboard"),
  profiles: () => invoke("profiles"),
  toolConnection: (profile) => invoke("tool_connection", { profile }),
  testConnection: (profile) => invoke("test_connection", { profile }),
  prepareShare: (profile, task) => invoke("prepare_shared_profile", { profile, task }),
  saveShare: (shared) => invoke("save_shared_profile", { shared }),
  openShare: () => invoke("open_shared_profile"),
  importSso: (shared) => invoke("import_shared_sso", { shared }),
  mfaDevices: (source) => invoke("mfa_devices", { source }),
  registerIam: (registration) => invoke("register_iam", { registration }),
  configureSso: () => invoke("configure_sso"),
  cli: (command) => invoke(command),
  session: (command, id) => invoke(command, { id }),
  saveSession: ({ id, label, login, task }) => invoke("save_session", { id, label, login, task }),
  renewMfa: (id, otp) => invoke("renew_mfa", { id, otp }),
  savePreferences: (preferences) => invoke("save_preferences", { preferences }),
  installCli: () => invoke("install_cli"),
  openDocs: (topic) => invoke("open_docs", { topic }),
  allowNotifications: async () => await isPermissionGranted() || await requestPermission() === "granted",
  onTrayEvent: (name, handle) => listen<string>(name, ({ payload }) => handle(payload)),
};
