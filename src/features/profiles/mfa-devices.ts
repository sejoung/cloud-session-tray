import { t } from "../../shared/i18n";
import { element, errorMessage } from "../../shared/html";

type Lookup = (source: string) => Promise<string[]>;

export function bindMfaLookup(form: HTMLFormElement, source: HTMLSelectElement, lookup: Lookup): void {
  const button = element<HTMLButtonElement>(form, '[data-device-lookup]');
  const device = element<HTMLInputElement>(form, 'input[name="device"]');
  const status = element<HTMLElement>(form, "[data-device-status]");
  const options = element<HTMLSelectElement>(form, "[data-device-options]");
  let revision = 0;

  function clearOptions(): void {
    options.hidden = true;
    options.replaceChildren(new Option(t("mfa.selectDevice"), ""));
  }

  function reset(): void {
    revision++;
    status.textContent = source.value ? "" : t("mfa.selectSourceFirst");
    button.disabled = !source.value;
    clearOptions();
  }

  function displayDevices(devices: string[], before: string): void {
    if (devices.length === 1 && device.value === before && !before) {
      device.value = devices[0];
      device.dataset.suggestion = device.value;
      status.textContent = t("mfa.deviceFilled");
      return;
    }
    for (const serial of devices) options.add(new Option(serial, serial));
    options.hidden = devices.length === 0;
    status.textContent = devices.length ? t("mfa.devicesFound") : t("mfa.devicesMissing");
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
    status.textContent = t("mfa.lookupPending");
    try {
      const devices = await lookup(profile);
      if (request === revision) displayDevices(devices, before);
    } catch (error) {
      if (request === revision) {
        status.textContent = errorMessage(error, "mfa.lookupFailed");
      }
    } finally {
      if (request === revision) button.disabled = false;
    }
  });
  reset();
}
