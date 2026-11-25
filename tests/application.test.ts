import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { mountApplication } from "../src/app";
import { element } from "../src/ui/html";
import { backend, cli, mockSession, profile } from "./fixtures";

let dispose: (() => void) | undefined;

beforeAll(() => {
  // jsdom provides the dialog element but delegates its native modal behavior to browsers.
  HTMLDialogElement.prototype.showModal = function () { this.open = true; };
  HTMLDialogElement.prototype.close = function (value?: string) {
    if (!this.open) return;
    this.open = false;
    if (value !== undefined) this.returnValue = value;
    this.dispatchEvent(new Event("close"));
  };
});

afterEach(() => {
  dispose?.();
  dispose = undefined;
  document.body.replaceChildren();
});

function mount(api: Parameters<typeof mountApplication>[1]): void {
  const root = document.createElement("div");
  document.body.replaceChildren(root);
  dispose = mountApplication(root, api);
}

async function click(selector: string): Promise<void> {
  await vi.waitFor(() => {
    const button = element<HTMLButtonElement>(document, selector);
    expect(button.disabled).toBe(false);
    button.click();
  });
}

async function openEditor(): Promise<void> {
  await click('[data-action="add"]');
  await vi.waitFor(() => expect(document.querySelector("#editor[open]")).not.toBeNull());
}

function field<T extends HTMLInputElement | HTMLSelectElement>(name: string): T {
  return element<T>(document, `#editor [name="${name}"]`);
}

function submit(form: string): void {
  element<HTMLFormElement>(document, form).requestSubmit();
}

describe("AWS profile imports", () => {
  it("fills the current IAM profile and MFA device, and avoids replacing an existing session", async () => {
    const device = "arn:aws:iam::123456789012:mfa/example";
    const { api } = backend({}, [
      profile("default", "iamSource", { preferred: true, mfaDevice: device }),
      profile("default-session", "iamSession"),
      profile("team", "iamSource"),
    ]);
    mount(api);
    await openEditor();

    expect(field("source").value).toBe("default");
    expect(field("target").value).toBe("default-session-2");
    expect(field("device").value).toBe(device);
    submit("#session-form");

    await vi.waitFor(() => expect(api.saveSession).toHaveBeenCalledWith({
      id: null, label: "default-session-2",
      login: { kind: "iamMfa", source: "default", target: "default-session-2", device, duration: null },
    }));
    expect(api.session).not.toHaveBeenCalledWith("check_session", expect.any(String));
  });

  it("looks up missing MFA device information using the selected source", async () => {
    const { api } = backend({}, [profile("default", "iamSource", { preferred: true })]);
    api.mfaDevices.mockResolvedValue(["SERIAL01234"]);
    mount(api);
    await openEditor();
    expect(field("device").value).toBe("");
    const button = element<HTMLButtonElement>(document, "[data-device-lookup]");
    const source = field("source");
    source.value = "";
    source.dispatchEvent(new Event("change", { bubbles: true }));
    expect(button.disabled).toBe(true);
    expect(element(document, "#mfa-device-status").textContent).toBe("원본 프로필을 먼저 선택하세요.");
    source.value = "default";
    source.dispatchEvent(new Event("change", { bubbles: true }));
    expect(button.disabled).toBe(false);
    await click("[data-device-lookup]");
    await vi.waitFor(() => expect(field("device").value).toBe("SERIAL01234"));
    expect(api.mfaDevices).toHaveBeenCalledExactlyOnceWith("default");
  });

  it("preserves manual target values and ignores device results after a source change", async () => {
    const { api } = backend({}, [
      profile("default", "iamSource", { preferred: true }),
      profile("team", "iamSource", { mfaDevice: "TEAM_SERIAL" }),
    ]);
    const pending = Promise.withResolvers<string[]>();
    api.mfaDevices.mockReturnValue(pending.promise);
    mount(api);
    await openEditor();
    field("target").value = "custom";
    await click("[data-device-lookup]");
    const source = field("source");
    source.value = "team";
    source.dispatchEvent(new Event("change", { bubbles: true }));
    pending.resolve(["OLD_SERIAL"]);
    await pending.promise;
    expect(field("device").value).toBe("TEAM_SERIAL");
    expect(field("target").value).toBe("custom");
  });

  it("registers a new IAM source, clears the key fields, and selects the created profile", async () => {
    const { api } = backend();
    api.registerIam.mockImplementation(async () => {
      for (const name of ["accessKeyId", "secretAccessKey"]) {
        const input = element<HTMLInputElement>(document, `#registration input[name="${name}"]`);
        expect(input.type).toBe("password");
        expect(input.value).toBe("");
      }
      return [profile("new-iam", "iamSource")];
    });
    mount(api);
    await openEditor();
    await click('[data-action="editor-register-iam"]');
    const fields = {
      name: "new-iam", accessKeyId: "AKIAEXAMPLE000000000",
      secretAccessKey: "synthetic-secret-for-tests-only", region: "ap-northeast-2",
    };
    for (const [name, value] of Object.entries(fields)) {
      element<HTMLInputElement>(document, `#registration input[name="${name}"]`).value = value;
    }
    submit("#registration-form");

    await vi.waitFor(() => expect(field("source").value).toBe("new-iam"));
    expect(field("target").value).toBe("new-iam-session");
    expect(document.querySelector("#registration[open]")).toBeNull();
    expect(api.registerIam).toHaveBeenCalledExactlyOnceWith(fields);
  });

  it("verifies the selected Identity Center profile after import without opening browser login", async () => {
    const { api, dashboard } = backend({}, [
      profile("team-dev", "sso", { preferred: true, ssoSession: "team" }),
      profile("default", "iamSource"),
    ]);
    api.session.mockImplementation(async (action) => {
      expect(action).toBe("check_session");
      dashboard.sessions[0].identity = {
        Account: "123456789012", Arn: "arn:aws:iam::123456789012:role/Developer", UserId: "fixture",
      };
      dashboard.sessions[0].status = "available";
    });
    mount(api);
    await openEditor();
    expect(field("kind").value).toBe("sso");
    expect(field("profile").value).toBe("team-dev");
    submit("#session-form");

    await vi.waitFor(() => expect(document.querySelector(".detail")?.textContent).toContain(":role/Developer"));
    expect(api.session).toHaveBeenCalledExactlyOnceWith("check_session", "saved");
  });
});

describe("Desktop controls", () => {
  it("opens setup and redetects AWS CLI after the installer is opened", async () => {
    const { api, dashboard } = backend({ cli: null, scanning: false }, [profile("default", "iamSource")]);
    api.cli.mockImplementation(async () => { dashboard.cli = cli; });
    mount(api);
    await click('.onboarding [data-page="cli"]');
    await click('[data-action="install"]');
    await vi.waitFor(() => expect(document.querySelector("#content")?.textContent).toContain("설치 창에서 설치를 진행"));
    await click('[data-action="rescan"]');
    await vi.waitFor(() => expect(document.querySelector("#content")?.textContent).toContain("/local/bin/aws"));
    expect(api.installCli).toHaveBeenCalledOnce();
    expect(api.cli).toHaveBeenCalledWith("rescan_cli");
  });

  it("masks and clears a tray renewal code before passing it to the backend", async () => {
    const session = mockSession("work-id", "Work", {
      kind: "iamMfa", source: "default", target: "work", device: "SERIAL01234", duration: null,
    });
    const { api, emit } = backend({ sessions: [session] });
    api.renewMfa.mockImplementation(async () => {
      expect(element<HTMLInputElement>(document, '#otp input[name="otp"]').value).toBe("");
    });
    mount(api);
    emit("request-otp", session.id);

    await vi.waitFor(() => expect(document.querySelector("#otp[open]")).not.toBeNull());
    const input = element<HTMLInputElement>(document, '#otp input[name="otp"]');
    expect(input.type).toBe("password");
    input.value = "123456";
    submit("#otp form");
    await vi.waitFor(() => expect(document.querySelector("#otp[open]")).toBeNull());

    expect(api.renewMfa).toHaveBeenCalledExactlyOnceWith(session.id, "123456");
    expect(document.body.textContent).not.toContain("123456");
  });

  it("saves expiration notifications only when permission is available", async () => {
    const { api } = backend();
    api.allowNotifications.mockResolvedValueOnce(false);
    mount(api);
    await click('nav [data-page="settings"]');
    const input = element<HTMLInputElement>(document, '#preferences [name="notify"]');
    input.click();
    submit("#preferences");

    await vi.waitFor(() => expect(document.querySelector("#flash")?.textContent).toContain("시스템 알림 권한"));
    expect(api.savePreferences).not.toHaveBeenCalled();

    submit("#preferences");
    await vi.waitFor(() => expect(api.savePreferences).toHaveBeenCalledWith({ notify: true, notifyMinutes: 10 }));
  });
});

describe("External tool profile workflows", () => {
  it("opens the existing destination from the AWS file inventory", async () => {
    const { api } = backend({}, [
      profile("default", "iamSource", { preferred: true, mfaDevice: "SERIAL", linkedTarget: "tech" }),
      profile("tech", "iamSession", { inCredentials: true, linkedSource: "default" }),
    ]);
    mount(api);
    await click('nav [data-page="cli"]');
    await click('[data-profile="tech"][data-workflow="adopt"]');
    await vi.waitFor(() => expect(document.querySelector("#editor[open]")).not.toBeNull());
    expect(field("source").value).toBe("default");
    expect(field("target").value).toBe("tech");
    expect(field("region").value).toBe("ap-northeast-2");
    const profileRow = document.querySelector('[data-profile="tech"]')?.closest("li");
    expect(profileRow?.textContent).toContain("config · credentials");
  });

  it("includes the exact destination and task in the export guide", async () => {
    const { api } = backend({ sessions: [mockSession("work-id", "Team dev", {
      kind: "iamMfa", source: "default", target: "tech", device: "SERIAL", duration: 43200,
    })] });
    mount(api);
    await click('[data-workflow="share"]');
    await vi.waitFor(() => expect(document.querySelector("#workflow[open]")).not.toBeNull());
    const task = element<HTMLTextAreaElement>(document, '#workflow [name="task"]');
    task.value = "Check the deployment";
    task.dispatchEvent(new Event("input", { bubbles: true }));
    const guide = element<HTMLTextAreaElement>(document, "[data-guide]").value;
    expect(guide).toContain("원본 프로필 default");
    expect(guide).toContain("갱신 대상 tech");
    expect(guide).toContain("Check the deployment");
    submit("#workflow form");
    await vi.waitFor(() => expect(api.saveShare).toHaveBeenCalledWith(expect.objectContaining({
      profile: "tech", task: "Check the deployment",
    })));
  });

  it("keeps the shared destination and task when a recipient uses their own source", async () => {
    const { api } = backend({}, [
      profile("default", "iamSource", { preferred: true, mfaDevice: "LOCAL_SERIAL" }),
    ]);
    api.openShare.mockResolvedValue({
      version: 1, profile: "tech", label: "Team dev", task: "Check the deployment", region: "ap-northeast-2",
      authentication: { kind: "iamMfa", sourceHint: "sender-source", duration: 43200 },
    });
    mount(api);
    await click('[data-workflow="import"]');
    await vi.waitFor(() => expect(document.querySelector("#workflow[open]")).not.toBeNull());
    submit("#workflow form");
    await vi.waitFor(() => expect(document.querySelector("#editor[open]")).not.toBeNull());
    expect(field("source").value).toBe("default");
    expect(field("target").value).toBe("tech");
    expect(field("device").value).toBe("LOCAL_SERIAL");
    expect(field("duration").value).toBe("43200");
    submit("#session-form");
    await vi.waitFor(() => expect(api.saveSession).toHaveBeenCalledWith(expect.objectContaining({
      task: "Check the deployment",
      login: { kind: "iamMfa", source: "default", target: "tech", device: "LOCAL_SERIAL", duration: 43200, region: "ap-northeast-2" },
    })));
  });

  it("uses the exact profile and file paths for the EKS command and diagnostic", async () => {
    const { api } = backend({ sessions: [mockSession("work-id", "Team dev", { kind: "sso", profile: "tech" })] });
    mount(api);
    await click('[data-workflow="connect"]');
    await vi.waitFor(() => expect(document.querySelector("#workflow[open]")).not.toBeNull());
    const name = element<HTMLInputElement>(document, "[data-cluster]");
    name.value = "team-dev";
    name.dispatchEvent(new Event("input", { bubbles: true }));
    const commands = element<HTMLTextAreaElement>(document, "[data-commands]").value;
    expect(commands).toContain("AWS_PROFILE='tech'");
    expect(commands).toContain("AWS_SHARED_CREDENTIALS_FILE='/test/credentials'");
    expect(commands).toContain("eks update-kubeconfig --name='team-dev' --region='ap-northeast-2' --profile='tech'");
    await click("#workflow [data-test]");
    expect(api.testConnection).toHaveBeenCalledExactlyOnceWith("tech");
  });
});
