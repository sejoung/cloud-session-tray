import type { DesktopApi } from "../desktop/api";
import type { Profile, ProfilePreset, SharedProfile } from "../shared/domain";
import { showImportDialog, showShareDialog } from "../features/sharing/dialog";

type WorkflowCallbacks = {
  openPreset: (preset: ProfilePreset) => Promise<void>;
  showImported: (id: string) => Promise<void>;
};

type WorkflowContext = {
  profile?: string;
  local?: Profile;
  task?: string;
  sessionId?: string;
};

export class ProfileWorkflows {
  constructor(
    private readonly api: DesktopApi,
    private readonly dialog: HTMLDialogElement,
    private readonly callbacks: WorkflowCallbacks,
  ) {}

  async perform(action: string, context: WorkflowContext): Promise<void> {
    switch (action) {
      case "import": await this.importProfile(); return;
      case "adopt": await this.adoptProfile(context); return;
      case "export": {
        if (!context.profile) return;
        const shared = await this.api.prepareShare(context.profile, context.task ?? "");
        showShareDialog(this.dialog, shared, (value) => this.api.saveShare(value));
        return;
      }
    }
  }

  private async adoptProfile({ local, sessionId }: WorkflowContext): Promise<void> {
    if (sessionId) {
      await this.callbacks.showImported(sessionId);
    } else if (local?.kind === "sso") {
      await this.callbacks.openPreset({ kind: "sso", profile: local.name });
    } else if (local?.targetAllowed) {
      await this.callbacks.openPreset({
        kind: "iamMfa", target: local.name, sourceHint: local.linkedSource, region: local.region,
      });
    }
  }

  private async importProfile(): Promise<void> {
    const shared = await this.api.openShare();
    if (!shared) return;
    showImportDialog(this.dialog, shared, (value) => this.applyShare(value));
    if (!this.dialog.open) this.dialog.showModal();
  }

  private async applyShare(shared: SharedProfile): Promise<void> {
    if (shared.authentication.kind === "sso") {
      const id = await this.api.importSso(shared);
      await this.callbacks.showImported(id);
    } else {
      await this.callbacks.openPreset({
        kind: "iamMfa", target: shared.profile, label: shared.label, task: shared.task,
        sourceHint: shared.authentication.sourceHint, duration: shared.authentication.duration, region: shared.region,
      });
    }
  }
}
