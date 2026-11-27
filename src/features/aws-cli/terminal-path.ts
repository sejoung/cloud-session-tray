import { localizeMessage } from "../../shared/i18n";
import type { Dashboard } from "../../shared/domain";
import { notice, text } from "../../shared/html";

export function terminalPathCard(state: Dashboard, busy: boolean): string {
  const { enabled, supported, error } = state.terminalPath;
  if (!enabled && (!supported || !state.cli?.v2)) return "";
  return `<section class="card section-card" aria-labelledby="terminal-path-title">
    <div class="card-heading">
      <h2 id="terminal-path-title">${text("cli.terminalPath")}</h2>
      ${enabled && !error ? `<span class="badge available"><span class="dot"></span>${text("cli.terminalPathEnabled")}</span>` : ""}
    </div>
    <p class="muted">${text("cli.terminalPathHelp")}</p>
    ${error ? notice(localizeMessage(error), true) : ""}
    <div class="actions">
      <button class="button" data-action="terminal-path" ${busy || state.scanning ? "disabled" : ""}>
        ${text(enabled ? "cli.terminalPathRemove" : "cli.terminalPathApply")}
      </button>
    </div>
  </section>`;
}
