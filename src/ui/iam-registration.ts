import type { IamRegistration } from "../domain";
import { bindSubmit, element } from "./html";
import { bindRegionField, regionField } from "./region-field";

export function showIamRegistration(
  dialog: HTMLDialogElement,
  region: string,
  register: (registration: IamRegistration) => Promise<void>,
): void {
  if (dialog.open) return;
  dialog.innerHTML = `<form id="registration-form">
    <div class="dialog-heading">
      <h2 id="registration-title">IAM 원본 프로필 등록</h2>
      <button type="button" class="icon-button" data-close="registration" aria-label="닫기">×</button>
    </div>
    <p class="dialog-summary">MFA 세션에 사용할 장기 IAM 액세스 키를 AWS 공유 자격 증명 파일에 저장합니다.</p>
    <label class="field">AWS 프로필 이름
      <input name="name" required maxlength="256" autofocus />
    </label>
    <label class="field">액세스 키 ID
      <input name="accessKeyId" type="password" required minlength="16" maxlength="128" autocomplete="off" />
    </label>
    <label class="field">비밀 액세스 키
      <input name="secretAccessKey" type="password" required minlength="16" maxlength="256" autocomplete="off" />
    </label>
    ${regionField("기본 리전 (선택)", "리전 설정 안 함", region)}
    <p class="field-hint">AWS CLI 출력 형식의 기본값은 JSON입니다.</p>
    <p class="form-error" role="alert" hidden></p>
    <div class="actions dialog-actions">
      <button class="button primary" type="submit">등록</button>
      <button class="button" type="button" data-close="registration">취소</button>
    </div>
  </form>`;

  const form = element<HTMLFormElement>(dialog, "form");
  const readRegion = bindRegionField(form);
  const accessKeyId = element<HTMLInputElement>(form, '[name="accessKeyId"]');
  const secretAccessKey = element<HTMLInputElement>(form, '[name="secretAccessKey"]');
  const closeButtons = Array.from(form.querySelectorAll<HTMLButtonElement>("[data-close]"));
  const setPending = (pending: boolean) => {
    for (const button of closeButtons) button.disabled = pending;
  };
  dialog.oncancel = (event) => {
    if (closeButtons.some((button) => button.disabled)) event.preventDefault();
  };
  bindSubmit(form, async () => {
    const registration = {
      name: element<HTMLInputElement>(form, '[name="name"]').value.trim(),
      region: readRegion(),
      accessKeyId: accessKeyId.value.trim(),
      secretAccessKey: secretAccessKey.value.trim(),
    };
    accessKeyId.value = "";
    secretAccessKey.value = "";
    setPending(true);
    try {
      await register(registration);
      if (dialog.open) dialog.close();
    } finally {
      setPending(false);
    }
  });
  dialog.addEventListener("close", () => dialog.replaceChildren(), { once: true });
  dialog.showModal();
}
