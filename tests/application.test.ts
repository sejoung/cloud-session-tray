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
      return [profile("new-iam", "iamSource", { region: "us-gov-west-1" })];
    });
    mount(api);
    await openEditor();
    await click('[data-action="editor-register-iam"]');
    const fields = {
      name: "new-iam", accessKeyId: "AKIAEXAMPLE000000000",
      secretAccessKey: "synthetic-secret-for-tests-only", region: "us-gov-west-1",
    };
    for (const name of ["name", "accessKeyId", "secretAccessKey"] as const) {
      element<HTMLInputElement>(document, `#registration input[name="${name}"]`).value = fields[name];
    }
    const region = element<HTMLSelectElement>(document, '#registration select[name="region"]');
    const custom = element<HTMLInputElement>(document, '#registration input[name="customRegion"]');
    expect(custom.disabled).toBe(true);
    region.selectedIndex = region.options.length - 1;
    region.dispatchEvent(new Event("change"));
    expect(custom.disabled).toBe(false);
    expect(custom.required).toBe(true);
    custom.value = fields.region;
    submit("#registration-form");

    await vi.waitFor(() => expect(field("source").value).toBe("new-iam"));
    expect(field("target").value).toBe("new-iam-session");
    expect(document.querySelector("#registration[open]")).toBeNull();
    expect(api.registerIam).toHaveBeenCalledExactlyOnceWith(fields);
  });

  it("offers key registration on the first screen and continues to MFA setup with the selected region", async () => {
    const { api } = backend();
    api.registerIam.mockResolvedValue([profile("work-source", "iamSource", { mfaDevice: "SERIAL" })]);
    mount(api);
    await click('.onboarding .primary[data-action="start-iam"]');
    element<HTMLInputElement>(document, '#registration [name="name"]').value = "work-source";
    element<HTMLInputElement>(document, '#registration [name="accessKeyId"]').value = "AKIAEXAMPLE000000000";
    element<HTMLInputElement>(document, '#registration [name="secretAccessKey"]').value = "synthetic-secret-for-tests-only";
    const region = element<HTMLSelectElement>(document, '#registration [name="region"]');
    expect(region.value).toBe("");
    region.value = "ap-northeast-2";
    expect(region.selectedOptions[0].textContent).toContain("서울");
    submit("#registration-form");

    await vi.waitFor(() => expect(document.querySelector("#editor[open]")).not.toBeNull());
    expect(api.registerIam).toHaveBeenCalledExactlyOnceWith(expect.objectContaining({ region: "ap-northeast-2" }));
    expect(field("source").value).toBe("work-source");
    expect(field("region").value).toBe("");
    submit("#session-form");
    await vi.waitFor(() => expect(api.saveSession).toHaveBeenCalledExactlyOnceWith({
      id: null, label: "work-source-session",
      login: { kind: "iamMfa", source: "work-source", target: "work-source-session", device: "SERIAL", duration: null },
    }));
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
  it.each([
    ["2.9.10", "2.10.0", true],
    ["2.10.9", "2.10.10", true],
    ["2.10.10", "2.10.10", false],
    ["2.10.10", "2.10.9", false],
    ["2.10.10", null, false],
  ])("shows an update download for %s and published version %s only when newer", async (installed, published, update) => {
    const { api } = backend({ cli: { ...cli, version: installed } });
    api.latestCliVersion.mockResolvedValue(published);
    mount(api);
    await click('nav [data-page="cli"]');
    await vi.waitFor(() => {
      expect(api.latestCliVersion).toHaveBeenCalledExactlyOnceWith(false);
      expect(element<HTMLButtonElement>(document, '[data-action="check-cli-release"]').disabled).toBe(false);
    });

    const button = document.querySelector('[data-action="install"]');
    if (update) {
      expect(button?.textContent).toContain("공식 업데이트 열기");
      expect(element(document, "#content").textContent).toContain(`AWS CLI v${published} 업데이트가 있습니다.`);
    } else {
      expect(button).toBeNull();
    }
    if (installed === published) expect(element(document, ".badge").textContent).toContain("최신 버전");
  });

  it("refreshes update availability on demand without downloading an installer", async () => {
    const { api } = backend();
    api.latestCliVersion.mockResolvedValueOnce(cli.version).mockResolvedValueOnce("2.50.0");
    mount(api);
    await click('nav [data-page="cli"]');
    await vi.waitFor(() => expect(element(document, ".badge").textContent).toContain("최신 버전"));
    await click('[data-action="check-cli-release"]');

    await vi.waitFor(() => expect(element(document, '[data-action="install"]').textContent).toContain("공식 업데이트 열기"));
    expect(api.latestCliVersion.mock.calls).toEqual([[false], [true]]);
    expect(api.installCli).not.toHaveBeenCalled();
  });

  it("waits for discovery before offering installation when only CLI v1 is present", async () => {
    const { api, dashboard } = backend({ cli: null, scanning: true });
    mount(api);
    await click('nav [data-page="cli"]');
    await vi.waitFor(() => expect(element(document, "#content").textContent).toContain("AWS CLI 찾는 중"));
    expect(document.querySelector('[data-action="install"]')).toBeNull();

    dashboard.cli = { ...cli, version: "1.44.0", v2: false, mfaSupported: false };
    dashboard.scanning = false;
    window.dispatchEvent(new Event("focus"));
    await vi.waitFor(() => expect(element(document, '[data-action="install"]').textContent).toContain("공식 설치 파일 받기"));
    expect(api.latestCliVersion).not.toHaveBeenCalled();
  });

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
    await vi.waitFor(() => expect(api.savePreferences).toHaveBeenCalledWith({ notify: true, notifyMinutes: 10, language: "ko" }));
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
    const { api } = backend({ sessions: [
      mockSession("other-id", "Other profile", { kind: "sso", profile: "other" }),
      mockSession("work-id", "Team dev", {
        kind: "iamMfa", source: "default", target: "tech", device: "SERIAL", duration: 43200,
      }),
    ] });
    mount(api);
    await click('[data-session="work-id"]');
    await click('.page-header [data-workflow="export"]');
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

  it("exports the local profile whose row was clicked", async () => {
    const { api } = backend({}, [profile("local-sso", "sso")]);
    mount(api);
    await click('nav [data-page="cli"]');
    await click('[data-profile="local-sso"][data-workflow="export"]');
    expect(api.prepareShare).toHaveBeenCalledExactlyOnceWith("local-sso", "");
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
