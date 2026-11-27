import { t, useLanguage } from "./ui/i18n";
import type { DesktopApi } from "./desktop-api";
import { profileName, selectSession, type Dashboard, type Login, type Page, type Preferences, type Profile, type ProfilePreset, type Session } from "./domain";
import { cliPage, type InstallerState } from "./ui/cli";
import { confirmRemoval, showOtp } from "./ui/confirmations";
import { showEditor } from "./ui/editor";
import { errorMessage, escape, text } from "./ui/html";
import { showIamRegistration } from "./ui/iam-registration";
import { sessionsPage } from "./ui/sessions";
import { readPreferences, settingsPage } from "./ui/settings";
import { ProfileWorkflows } from "./profile-workflows";
import { buildShell } from "./ui/shell";

class Application {
  private readonly ui: ReturnType<typeof buildShell>;
  private readonly workflows: ProfileWorkflows;
  private readonly lifecycle = new AbortController();
  private readonly listenerCleanup: ReturnType<DesktopApi["onTrayEvent"]>[] = [];
  private dashboard: Dashboard | null = null;
  private page: Page = "sessions";
  private selectedId: string | null = null;
  private profiles: Profile[] | null = null;
  private profileRequest: Promise<Profile[]> | null = null;
  private dashboardRequest: Promise<void> | null = null;
  private cliBusy = false;
  private installerState: InstallerState = "idle";
  private latestCliVersion: string | null = null;
  private checkingCliRelease = false;
  private wizardBusy = false;
  private editedSession?: Session;
  private preferencesDraft: Preferences | null = null;
  private flashTimeout?: number;
  private pollTimer?: number;

  constructor(private readonly root: HTMLElement, private readonly api: DesktopApi) {
    useLanguage();
    this.ui = buildShell(root);
    this.workflows = new ProfileWorkflows(api, this.ui.workflow, {
      openPreset: (preset) => this.openPreset(preset),
      showImported: async (id) => {
        await this.selectFromTray(id);
        if (this.dashboard?.cli?.v2) await this.loadProfiles();
      },
    });
  }

  start(): void {
    const { content, otpDialog } = this.ui;
    const { signal } = this.lifecycle;
    this.root.addEventListener("click", this.handleClick, { signal });
    content.addEventListener("input", (event) => {
      const form = (event.target as HTMLElement).closest<HTMLFormElement>("#preferences");
      if (form) this.preferencesDraft = readPreferences(form);
    }, { signal });
    content.addEventListener("submit", (event) => {
      const form = event.target as HTMLFormElement;
      if (form.id !== "preferences") return;
      event.preventDefault();
      void this.savePreferences(form).catch(this.reportError);
    }, { signal });
    otpDialog.addEventListener("close", () => otpDialog.replaceChildren(), { signal });
    window.addEventListener("focus", () => {
      void this.checkCliRelease();
      const task = this.installerState === "opened" ? this.checkCli("rescan_cli") : this.refresh();
      void task.catch(this.reportError);
    }, { signal });
    this.bindTrayEvents();
    this.render();
    void this.refresh().catch(this.reportError);
    this.pollTimer = window.setInterval(() => {
      if (!document.hidden) void this.refresh().catch(() => {});
    }, 1000);
  }

  dispose(): void {
    this.lifecycle.abort();
    window.clearInterval(this.pollTimer);
    window.clearTimeout(this.flashTimeout);
    for (const cleanup of this.listenerCleanup) {
      void cleanup.then((stop) => stop()).catch(() => {});
    }
  }

  private bindTrayEvents(): void {
    this.listenerCleanup.push(
      this.api.onTrayEvent("select-session", (id) => {
        void this.selectFromTray(id).catch(this.reportError);
      }),
      this.api.onTrayEvent("request-otp", (id) => {
        void this.selectFromTray(id).then((session) => {
          if (session) this.requestOtp(session);
        }).catch(this.reportError);
      }),
      this.api.onTrayEvent("session-error", this.reportError),
    );
    for (const registration of this.listenerCleanup) void registration.catch(this.reportError);
  }

  private selectedSession(): Session | undefined {
    return selectSession(this.dashboard, this.selectedId);
  }

  private showMessage(message: string, error = false): void {
    const { flash } = this.ui;
    flash.textContent = message;
    flash.classList.toggle("flash-error", error);
    flash.hidden = false;
    window.clearTimeout(this.flashTimeout);
    this.flashTimeout = window.setTimeout(() => { flash.hidden = true; }, error ? 9500 : 4500);
  }

  private reportError = (error: unknown): void => {
    this.showMessage(errorMessage(error), true);
  };

  private pageHtml(state: Dashboard): string {
    switch (this.page) {
      case "sessions": return sessionsPage(state, this.profiles, this.selectedSession());
      case "cli": return cliPage(state, {
        profiles: this.profiles, cliBusy: this.cliBusy,
        installerState: this.installerState, wizardBusy: this.wizardBusy,
        latestCliVersion: this.latestCliVersion, checkingCliRelease: this.checkingCliRelease,
      });
      case "settings": return settingsPage(state, this.preferencesDraft);
    }
  }

  private render(): void {
    const { content, railStatus } = this.ui;
    for (const button of this.root.querySelectorAll<HTMLButtonElement>("nav [data-page]")) {
      if (button.dataset.page === this.page) button.setAttribute("aria-current", "page");
      else button.removeAttribute("aria-current");
    }
    if (!this.dashboard) {
      content.innerHTML = `<p class="initial-loading" role="status">${text("app.loading")}</p>`;
      return;
    }
    const { cli, scanning } = this.dashboard;
    const cliStatus = scanning ? t("app.checkingCli") : cli?.v2 ? `AWS CLI ${escape(cli.version)}` : t("app.cliRequired");
    railStatus.innerHTML = `<span class="dot ${cli?.v2 ? "online" : ""}"></span><span>${cliStatus}</span>`;

    const active = content.contains(document.activeElement) ? document.activeElement as HTMLElement : null;
    const next = this.pageHtml(this.dashboard);
    if (next === content.dataset.rendered) return;
    if (this.page === "settings" && active?.closest("#preferences") && this.preferencesDraft) return;
    content.dataset.rendered = next;
    content.innerHTML = next;
    this.restoreFocus(active);
  }

  private restoreFocus(active: HTMLElement | null): void {
    const key = active?.dataset.action ? "action" : "session";
    const value = active?.dataset[key];
    if (!value) return;
    const buttons = this.ui.content.querySelectorAll<HTMLElement>(`[data-${key}]`);
    Array.from(buttons).find((button) => button.dataset[key] === value && button.dataset.path === active?.dataset.path)?.focus({ preventScroll: true });
  }

  private navigate(next: Page): void {
    if (this.page === next) return;
    this.page = next;
    this.render();
    this.ui.content.focus();
    void this.checkCliRelease();
  }

  private refresh(): Promise<void> {
    this.dashboardRequest ??= this.api.dashboard().then((state) => {
      const previous = this.dashboard?.cli;
      this.dashboard = state;
      if (useLanguage(state.preferences.language)) this.ui.translate();
      if (state.cli?.path !== previous?.path || state.cli?.version !== previous?.version) {
        this.profiles = null;
        if (state.cli?.v2) void this.loadProfiles().catch(() => {});
        void this.checkCliRelease();
      }
      this.render();
    }).finally(() => { this.dashboardRequest = null; });
    return this.dashboardRequest;
  }

  private loadProfiles(): Promise<Profile[]> {
    this.profileRequest ??= this.api.profiles().then((items) => {
      this.profiles = items;
      this.render();
      return items;
    }).finally(() => { this.profileRequest = null; });
    return this.profileRequest;
  }

  private async openEditor(session?: Session, defaultKind?: Login["kind"], preferredSource?: string, preset?: ProfilePreset): Promise<void> {
    const items = preferredSource && this.profiles ? this.profiles : await this.loadProfiles();
    this.editedSession = session;
    showEditor(this.ui.editor, {
      items, session, defaultKind, preferredSource, preset,
      managed: new Set(this.dashboard?.sessions.map((item) => profileName(item.login))),
      lookupMfaDevices: this.api.mfaDevices,
      onSave: async (draft) => {
        const id = await this.api.saveSession(draft);
        this.selectedId = id;
        if (!session && (draft.login.kind === "sso" || items.some((profile) => {
          return profile.name === profileName(draft.login) && profile.kind === "iamSession";
        }))) {
          await this.api.session("check_session", id).catch(this.reportError);
        }
        this.navigate("sessions");
        await this.refresh();
        this.render();
      },
    });
  }

  private requestOtp(session: Session): void {
    if (!this.dashboard?.cli?.mfaSupported) {
      this.showMessage(t("errors.mfaCliRequired"), true);
      return;
    }
    if (session.phase) return;
    if (this.ui.editor.open) this.ui.editor.close();
    showOtp(this.ui.otpDialog, session, async (code) => {
      await this.api.renewMfa(session.id, code);
      await this.refresh();
    });
  }

  private async remove(session: Session): Promise<void> {
    if (!await confirmRemoval(this.ui.confirmDialog, session)) return;
    await this.api.session("remove_session", session.id);
    this.ui.editor.close();
    this.selectedId = null;
    await this.refresh();
  }

  private openIamRegistration(continueToEditor: boolean, session?: Session): void {
    const region = this.profiles?.find((profile) => profile.preferred)?.region ?? "";
    showIamRegistration(this.ui.registration, region, async (registration) => {
      const profiles = await this.api.registerIam(registration);
      await this.profileRequest?.catch(() => {});
      this.profiles = profiles;
      if (continueToEditor) await this.openEditor(session, "iamMfa", registration.name);
      this.render();
      this.showMessage(t("app.iamRegistered"));
    });
  }

  private async runWizard(fromEditor: boolean): Promise<void> {
    if (this.wizardBusy || this.dashboard?.wizard) return;
    const session = fromEditor ? this.editedSession : undefined;
    if (fromEditor) this.ui.editor.close();
    this.wizardBusy = true;
    this.render();
    this.showMessage(t("app.completeSsoWizard"));
    try {
      this.profiles = await this.api.configureSso();
      if (fromEditor) await this.openEditor(session, "sso");
      else this.showMessage(t("app.ssoWizardFinished"));
    } catch (error) {
      await this.loadProfiles().catch(() => {});
      this.reportError(error);
    } finally {
      this.wizardBusy = false;
      await this.refresh();
      this.render();
    }
  }

  private async checkCliRelease(force = false): Promise<void> {
    if (this.page !== "cli" || !this.dashboard?.cli?.v2 || this.checkingCliRelease) return;
    this.checkingCliRelease = true;
    this.render();
    this.latestCliVersion = await this.api.latestCliVersion(force).catch(() => null);
    this.checkingCliRelease = false;
    if (force && !this.latestCliVersion) this.showMessage(t("app.releaseCheckFailed"), true);
    this.render();
  }

  private async checkCli(command: Parameters<DesktopApi["cli"]>[0], path?: string): Promise<void> {
    if (this.cliBusy) return;
    const previousVersion = this.dashboard?.cli?.version;
    this.cliBusy = true;
    this.render();
    try {
      const changed = path ? await this.api.cli(command, path) : await this.api.cli(command);
      await this.refresh();
      if (command === "remove_cli" && changed) this.showMessage(t("app.cliRemoved"));
      if (this.dashboard?.cli?.v2 && changed !== false) {
        if (this.installerState === "opened" && this.dashboard.cli.version !== previousVersion) this.installerState = "idle";
        await this.loadProfiles();
      } else if (this.installerState === "opened" && command === "rescan_cli") {
        this.showMessage(t("app.installerNotDetected"), true);
      }
    } finally {
      this.cliBusy = false;
      this.render();
    }
  }

  private async install(): Promise<void> {
    if (this.installerState === "downloading") return;
    this.installerState = "downloading";
    this.render();
    try {
      await this.api.installCli();
      this.installerState = "opened";
    } catch (error) {
      this.installerState = "idle";
      throw error;
    } finally {
      this.render();
    }
  }

  private async savePreferences(form: HTMLFormElement): Promise<void> {
    const preferences = readPreferences(form);
    if (preferences.notify && !this.dashboard?.preferences.notify && !await this.api.allowNotifications()) {
      this.showMessage(t("app.notificationPermission"), true);
      return;
    }
    await this.api.savePreferences(preferences);
    this.preferencesDraft = null;
    await this.dashboardRequest?.catch(() => {});
    await this.refresh();
    this.showMessage(t("app.preferencesSaved"));
  }

  private async perform(action: string, path?: string): Promise<void> {
    const session = this.selectedSession();
    switch (action) {
      case "add": await this.openEditor(); return;
      case "edit": if (session) await this.openEditor(session); return;
      case "remove": if (session) await this.remove(session); return;
      case "renew":
        if (session?.login.kind === "iamMfa") this.requestOtp(session);
        else if (session) await this.api.session("renew_sso", session.id);
        break;
      case "check": if (session) await this.api.session("check_session", session.id); break;
      case "cancel": if (session) await this.api.session("cancel_session", session.id); break;
      case "install": await this.install(); return;
      case "open-logs": await this.api.openLogFolder(); return;
      case "check-cli-release": await this.checkCliRelease(true); return;
      case "rescan": await this.checkCli("rescan_cli"); return;
      case "choose": await this.checkCli("choose_cli", path); return;
      case "remove-cli": await this.checkCli("remove_cli", path); return;
      case "terminal-path": await this.checkCli(this.dashboard?.terminalPath.enabled ? "disable_terminal_path" : "enable_terminal_path"); return;
      case "auto-cli": await this.checkCli("reset_cli"); return;
      case "profiles": await this.loadProfiles(); break;
      case "register-iam": this.openIamRegistration(false); return;
      case "start-iam": this.openIamRegistration(true); return;
      case "editor-register-iam": this.openIamRegistration(true, this.editedSession); return;
      case "wizard": await this.runWizard(false); return;
      case "editor-wizard": await this.runWizard(true); return;
    }
    await this.refresh();
  }

  private handleClick = (event: MouseEvent): void => {
    const button = (event.target as HTMLElement | null)?.closest<HTMLButtonElement>("button");
    if (!button) return;
    const { page, session, close, docs, action, workflow, profile, path } = button.dataset;
    if (page) this.navigate(page as Page);
    if (session) {
      this.selectedId = session;
      this.render();
    }
    if (close) this.root.querySelector<HTMLDialogElement>(`#${close}`)?.close();
    if (docs) void this.api.openDocs(docs).catch(this.reportError);
    if (action) void this.perform(action, path).catch(this.reportError);
    if (workflow) void this.performWorkflow(workflow, profile).catch(this.reportError);
  };

  private openPreset(preset: ProfilePreset): Promise<void> {
    const profile = preset.kind === "sso" ? preset.profile : preset.target;
    const saved = this.dashboard?.sessions.find((item) => profileName(item.login) === profile);
    return this.openEditor(saved, undefined, undefined, preset);
  }

  private performWorkflow(action: string, name?: string): Promise<void> {
    const selected = this.selectedSession();
    const profile = name ?? (selected ? profileName(selected.login) : undefined);
    const saved = this.dashboard?.sessions.find((item) => profileName(item.login) === profile);
    return this.workflows.perform(action, {
      profile, task: saved?.task, sessionId: saved?.id,
      local: this.profiles?.find((item) => item.name === profile),
    });
  }

  private async selectFromTray(id: string): Promise<Session | undefined> {
    await this.refresh();
    this.selectedId = id;
    this.navigate("sessions");
    this.render();
    return this.dashboard?.sessions.find((session) => session.id === id);
  }
}

export function mountApplication(root: HTMLElement, api: DesktopApi): () => void {
  const app = new Application(root, api);
  app.start();
  return () => app.dispose();
}
