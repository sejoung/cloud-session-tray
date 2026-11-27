import { expect, test, type Locator, type Page } from "@playwright/test";
import { fileURLToPath } from "node:url";
import en from "../../locales/en.json" with { type: "json" };
import ko from "../../locales/ko.json" with { type: "json" };

const fixture = "/tests/e2e/fixture.html";
const isCapture = () => test.info().project.name.startsWith("manual-");
const isEnglish = () => test.info().project.name === "manual-en";
const copy = () => isEnglish() ? en : ko;

async function capture(page: Page, name: string, next?: Locator): Promise<void> {
  if (!isCapture()) return;
  await next?.evaluate((element) => element.setAttribute("data-manual-focus", ""));
  try {
    await page.screenshot({
      path: fileURLToPath(new URL(`../../docs/assets/${isEnglish() ? "en/" : ""}${name}.png`, import.meta.url)),
      animations: "disabled",
      style: `:focus-visible { outline: none !important; }
        .flash:not(.flash-error) { visibility: hidden; }
        [data-manual-focus] { outline: 2px solid #4873ef !important; outline-offset: 3px !important; }`,
    });
  } finally {
    await next?.evaluate((element) => element.removeAttribute("data-manual-focus"));
  }
}

test("connects and renews an IAM session, then exports and imports its settings", async ({ page }) => {
  const labels = copy();
  const task = isEnglish() ? "Check AWS resources in the development account" : "개발 계정의 AWS 리소스 확인";
  await page.goto(fixture);
  const exportFile = page.getByRole("button", { name: labels["share.export"], exact: true });
  const importFile = page.getByRole("button", { name: labels["common.importShare"], exact: true });
  await expect(exportFile).toBeDisabled();
  await expect(importFile).toBeEnabled();
  await page.getByRole("button", { name: labels["sessions.openCli"] }).click();
  const installer = page.getByRole("button", { name: labels["cli.download"] });
  await expect(installer).toBeVisible();
  await capture(page, "cli", installer);
  await installer.click();
  await expect(page.getByText(labels["cli.installerOpened"])).toBeVisible();
  await page.getByRole("button", { name: labels["cli.rescan"] }).click();
  await expect(page.getByText(labels["cli.upToDate"], { exact: true })).toBeVisible();
  await expect(installer).toHaveCount(0);

  await page.getByRole("button", { name: labels["common.sessions"], exact: true }).click();
  await page.getByRole("button", { name: labels["sessions.registerKeys"] }).click();
  const registration = page.getByRole("dialog", { name: labels["common.openIamRegistration"] });
  const register = registration.getByRole("button", { name: labels["common.register"], exact: true });
  await registration.getByLabel(labels["common.profileName"]).fill("work-source");
  await registration.getByLabel(labels["iam.defaultRegion"]).selectOption("ap-northeast-2");
  await capture(page, "iam-register", register);
  await registration.getByLabel(labels["iam.accessKeyId"]).fill("AKIAIOSFODNN7EXAMPLE");
  await registration.getByLabel(labels["iam.secretKey"]).fill("documentation-example-key");
  await register.click();

  const editor = page.getByRole("dialog", { name: labels["editor.addProfile"] });
  const findDevice = editor.getByRole("button", { name: labels["editor.findMfa"] });
  await expect(editor.getByRole("combobox", { name: labels["common.sourceProfile"], exact: true })).toHaveValue("work-source");
  await expect(editor.getByLabel(labels["common.targetProfile"])).toHaveValue("work-source-session");
  await expect(findDevice).toBeInViewport({ ratio: 1 });
  await capture(page, "iam-mfa", findDevice);
  await findDevice.click();
  await expect(editor.getByLabel(labels["editor.mfaDevice"])).toHaveValue("arn:aws:iam::123456789012:mfa/example");
  await editor.getByLabel(labels["editor.displayName"]).fill("Development");
  await editor.getByLabel(labels["common.optionalTask"]).fill(task);
  await editor.getByRole("button", { name: labels["common.save"] }).click();

  await page.getByRole("button", { name: labels["common.renew"], exact: true }).click();
  const renewal = page.getByRole("dialog", { name: labels["confirmation.renewMfa"] });
  const otp = renewal.getByLabel(labels["confirmation.mfaCode"]);
  await expect(otp).toBeFocused();
  await capture(page, "renew", otp);
  await otp.fill("123456");
  await renewal.getByRole("button", { name: labels["common.renew"] }).click();
  const session = page.getByRole("region", { name: labels["sessions.selected"] });
  await expect(session.getByText(labels["sessions.available"])).toBeVisible();
  await expect(session.getByText("123456789012", { exact: true })).toBeVisible();
  await capture(page, "sessions");

  await test.step("exports a share file and imports its settings", async () => {
    await exportFile.click();
    const share = page.getByRole("dialog", { name: labels["share.title"] });
    await expect(share.getByText(labels["share.credentialsExcluded"])).toBeInViewport({ ratio: 1 });
    await expect(share.getByLabel(labels["common.optionalTask"])).toHaveAccessibleDescription(labels["share.notePrivacy"]);
    const target = share.getByLabel(labels["common.targetProfile"]);
    await expect(target).toHaveValue("work-source-session");
    await target.fill("team-dev-session");
    const saveShare = share.getByRole("button", { name: labels["share.export"], exact: true });
    await capture(page, "share-export", saveShare);
    await saveShare.click();
    await expect(share.getByRole("status")).toHaveText(labels["share.exported"]);
    await share.getByRole("button", { name: labels["common.close"], exact: true }).click();

    await importFile.click();
    const preview = page.getByRole("dialog", { name: labels["share.importTitle"] });
    await expect(preview.getByLabel(labels["common.targetProfile"])).toHaveValue("team-dev-session");
    await expect(preview.getByText(task)).toBeVisible();
    const openIam = preview.getByRole("button", { name: labels["share.openIam"] });
    await capture(page, "share-import", openIam);
    await openIam.click();
    await expect(editor.getByLabel(labels["common.targetProfile"])).toHaveValue("team-dev-session");
    await expect(editor.getByLabel(labels["common.optionalTask"])).toHaveValue(task);
    await findDevice.click();
    await expect(editor.getByLabel(labels["editor.mfaDevice"])).toHaveValue("arn:aws:iam::123456789012:mfa/example");
    await editor.getByRole("button", { name: labels["common.save"] }).click();
    await expect(session.getByText("team-dev-session", { exact: true })).toBeVisible();
  });
});

test("connects an IAM Identity Center profile", async ({ page }) => {
  const labels = copy();
  await page.goto(`${fixture}?scenario=sso`);
  await page.getByRole("button", { name: labels["sessions.connectProfile"] }).click();
  const editor = page.getByRole("dialog", { name: labels["editor.addProfile"] });
  await expect(editor.getByLabel(labels["common.authentication"])).toHaveValue("sso");
  await expect(editor.getByLabel(labels["editor.ssoProfile"])).toHaveValue("team-dev");
  const wizard = editor.getByRole("button", { name: labels["common.ssoWizard"] });
  await expect(wizard).toBeVisible();
  await expect(editor.locator(".inline-help").getByRole("button", { name: labels["common.ssoWizard"] })).toHaveCount(0);
  await capture(page, "sso", wizard);
  await wizard.click();
  await expect(editor.getByLabel(labels["editor.ssoProfile"])).toHaveValue("team-dev");
  await editor.getByRole("button", { name: labels["common.save"] }).click();
  await expect(page.getByRole("region", { name: labels["sessions.selected"] }).getByText(labels["sessions.available"])).toBeVisible();
});
