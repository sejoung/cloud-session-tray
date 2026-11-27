import appIcon from "../../src-tauri/icons/icon.png";
import { element, text } from "./html";
import { t } from "./i18n";

export function buildShell(root: HTMLElement) {
  root.innerHTML = `
  <div class="shell">
    <aside class="rail">
      <div class="brand">
        <img src="${appIcon}" alt="" width="36" height="36" />
        <span>Cloud Session<br />Tray</span>
      </div>
      <nav aria-label="${text("shell.navigation")}">
        <button data-page="sessions"><span class="nav-icon session-icon" aria-hidden="true"></span><span data-nav-label>${text("common.sessions")}</span></button>
        <button data-page="cli"><span class="nav-icon terminal-icon" aria-hidden="true">&gt;_</span>AWS CLI</button>
        <button data-page="settings"><span class="nav-icon" aria-hidden="true">⚙</span><span data-nav-label>${text("common.settings")}</span></button>
      </nav>
      <button class="rail-status" data-page="cli" id="rail-status"></button>
    </aside>
    <main id="content" tabindex="-1"></main>
  </div>
  <div id="flash" class="flash" hidden role="status" aria-live="polite"></div>
  <dialog id="editor" aria-labelledby="editor-title"></dialog>
  <dialog id="otp" aria-labelledby="otp-title"></dialog>
  <dialog id="confirm" aria-labelledby="confirm-title"></dialog>
  <dialog id="registration" aria-labelledby="registration-title"></dialog>
  <dialog id="workflow" aria-labelledby="workflow-title"></dialog>
`;

  return {
    translate: () => {
      element(root, "nav").setAttribute("aria-label", t("shell.navigation"));
      element(root, '[data-page="sessions"] [data-nav-label]').textContent = t("common.sessions");
      element(root, '[data-page="settings"] [data-nav-label]').textContent = t("common.settings");
    },
    content: element<HTMLElement>(root, "#content"),
    railStatus: element<HTMLButtonElement>(root, "#rail-status"),
    editor: element<HTMLDialogElement>(root, "#editor"),
    otpDialog: element<HTMLDialogElement>(root, "#otp"),
    confirmDialog: element<HTMLDialogElement>(root, "#confirm"),
    registration: element<HTMLDialogElement>(root, "#registration"),
    workflow: element<HTMLDialogElement>(root, "#workflow"),
    flash: element<HTMLElement>(root, "#flash"),
  };
}
