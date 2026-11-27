import { localizeMessage, t } from "./i18n";
import type { Dashboard, Preferences } from "../domain";
import { header, notice, text } from "./html";

export function settingsPage(state: Dashboard, preferencesDraft: Preferences | null): string {
  const preferences = preferencesDraft ?? state.preferences;
  return header(t("common.settings"), t("settings.description"))
    + (state.settingsError ? notice(localizeMessage(state.settingsError), true) : "")
    + `<form id="preferences" class="settings-form">
      <section class="card section-card">
        <label class="field short-field">${text("settings.language")}
          <select name="language">
            <option value="">${text("settings.system")}</option>
            <option value="en" ${preferences.language === "en" ? "selected" : ""}>${text("settings.english")}</option>
            <option value="ko" ${preferences.language === "ko" ? "selected" : ""}>${text("settings.korean")}</option>
          </select>
        </label>
      </section>
      <section class="card section-card">
      <h2>${text("settings.notifications")}</h2>
      <p class="muted">${text("settings.notificationHelp")}</p>
        <label class="check-label"><input type="checkbox" name="notify" ${preferences.notify ? "checked" : ""} /> ${text("settings.notifyBefore")}</label>
        <label class="field short-field">${text("settings.notifyMinutes")}
          <input type="number" min="1" max="1440" step="1" required name="notifyMinutes" value="${preferences.notifyMinutes}" />
        </label>
        <div class="actions"><button class="button primary" type="submit">${text("common.save")}</button></div>
      </section>
    </form>
    <section class="card section-card">
      <h2>${text("settings.diagnostics")}</h2>
      <div class="actions"><button class="button" data-action="open-logs">${text("settings.logs")}</button></div>
    </section>`;
}

export function readPreferences(form: HTMLFormElement): Preferences {
  const data = new FormData(form);
  const language = data.get("language");
  return {
    notify: data.has("notify"),
    notifyMinutes: Number(data.get("notifyMinutes")),
    language: language === "en" || language === "ko" ? language : null,
  };
}
