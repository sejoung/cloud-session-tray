import type { Dashboard, Preferences } from "../domain";
import { header, notice } from "./html";

export function settingsPage(state: Dashboard, preferencesDraft: Preferences | null): string {
  const preferences = preferencesDraft ?? state.preferences;
  return header("설정", "AWS 세션 만료 알림을 관리하세요.")
    + (state.settingsError ? notice(state.settingsError, true) : "")
    + `<section class="card section-card">
      <h2>만료 알림</h2>
      <p class="muted">만료 시각을 확인한 IAM MFA 세션에 알림을 보냅니다.</p>
      <form id="preferences" class="settings-form">
        <label class="check-label"><input type="checkbox" name="notify" ${preferences.notify ? "checked" : ""} /> 만료 전에 알림 받기</label>
        <label class="field short-field">만료 전 알림 (분)
          <input type="number" min="1" max="1440" step="1" required name="notifyMinutes" value="${preferences.notifyMinutes}" />
        </label>
        <div class="actions"><button class="button primary" type="submit">저장</button></div>
      </form>
    </section>`;
}

export function readPreferences(form: HTMLFormElement): Preferences {
  const data = new FormData(form);
  return { notify: data.has("notify"), notifyMinutes: Number(data.get("notifyMinutes")) };
}
