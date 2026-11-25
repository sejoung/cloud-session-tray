import { element, errorMessage } from "./html";

type Lookup = (source: string) => Promise<string[]>;

export function bindMfaLookup(form: HTMLFormElement, source: HTMLSelectElement, lookup: Lookup): void {
  const button = element<HTMLButtonElement>(form, '[data-device-lookup]');
  const device = element<HTMLInputElement>(form, 'input[name="device"]');
  const status = element<HTMLElement>(form, "[data-device-status]");
  const options = element<HTMLSelectElement>(form, "[data-device-options]");
  let revision = 0;

  function clearOptions(): void {
    options.hidden = true;
    options.replaceChildren(new Option("조회한 MFA 장치 선택", ""));
  }

  function reset(): void {
    revision++;
    status.textContent = source.value ? "" : "원본 프로필을 먼저 선택하세요.";
    button.disabled = !source.value;
    clearOptions();
  }

  function displayDevices(devices: string[], before: string): void {
    if (devices.length === 1 && device.value === before && !before) {
      device.value = devices[0];
      device.dataset.suggestion = device.value;
      status.textContent = "등록된 MFA 장치를 채웠습니다.";
      return;
    }
    for (const serial of devices) options.add(new Option(serial, serial));
    options.hidden = devices.length === 0;
    status.textContent = devices.length ? "등록된 장치를 선택할 수 있습니다." : "등록된 장치를 찾지 못했습니다.";
  }

  source.addEventListener("change", reset);
  options.addEventListener("change", () => {
    if (options.value) device.value = options.value;
  });
  button.addEventListener("click", async () => {
    const profile = source.value;
    const before = device.value;
    if (!profile) return;
    const request = ++revision;
    clearOptions();
    button.disabled = true;
    status.textContent = "AWS에서 등록된 장치를 확인하는 중…";
    try {
      const devices = await lookup(profile);
      if (request === revision) displayDevices(devices, before);
    } catch (error) {
      if (request === revision) {
        status.textContent = errorMessage(error, "MFA 장치를 조회할 수 없습니다.");
      }
    } finally {
      if (request === revision) button.disabled = false;
    }
  });
  reset();
}
