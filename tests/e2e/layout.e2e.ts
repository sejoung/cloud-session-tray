import { expect, test, type Locator } from "@playwright/test";
import en from "../../locales/en.json" with { type: "json" };
import ko from "../../locales/ko.json" with { type: "json" };
import tauri from "../../src-tauri/tauri.conf.json" with { type: "json" };

const [{ width, height, minWidth, minHeight }] = tauri.app.windows;

async function expectNoHorizontalOverflow(container: Locator): Promise<void> {
  await expect.poll(() => container.evaluate((element) => element.scrollWidth - element.clientWidth)).toBeLessThanOrEqual(1);
}

async function expectDialogFits(dialog: Locator): Promise<void> {
  await expect(dialog).toBeInViewport({ ratio: 1 });
  await expect(dialog.locator(".dialog-heading")).toBeInViewport({ ratio: 1 });
  await expect(dialog.locator(".dialog-footer")).toBeInViewport({ ratio: 1 });
  await expectNoHorizontalOverflow(dialog.locator(".dialog-body"));
}

for (const [locale, labels] of [["ko-KR", ko], ["en-US", en]] as const) {
  for (const viewport of [{ width, height }, { width: minWidth, height: minHeight }]) {
    test.describe(`${locale} at ${viewport.width}×${viewport.height}`, () => {
      test.use({ locale, viewport });

      test("fits desktop pages and keeps dialog actions in view while their content scrolls", async ({ page }) => {
        await page.goto("/tests/e2e/fixture.html?scenario=layout");
        const pageContent = page.getByRole("main");
        await expect(pageContent.getByRole("heading", { name: labels["sessions.title"] })).toBeVisible();
        await expectNoHorizontalOverflow(pageContent);
        await expectNoHorizontalOverflow(pageContent.locator(".session-list"));
        await expectNoHorizontalOverflow(page.locator("html"));

        await page.getByRole("button", { name: labels["common.settings"], exact: true }).click();
        await expect(pageContent.getByRole("button", { name: labels["settings.save"] })).toBeInViewport({ ratio: 1 });
        await expectNoHorizontalOverflow(pageContent);
        await page.getByRole("button", { name: "AWS CLI", exact: true }).click();
        await expect(pageContent.getByRole("heading", { name: labels["cli.localProfiles"] })).toBeVisible();
        await expectNoHorizontalOverflow(pageContent);
        await expectNoHorizontalOverflow(pageContent.locator(".catalog"));
        await page.getByRole("button", { name: labels["cli.registerIam"] }).click();

        const registration = page.getByRole("dialog", { name: labels["common.openIamRegistration"] });
        await registration.getByLabel(labels["iam.defaultRegion"]).selectOption("__custom__");
        await expectDialogFits(registration);
        await registration.getByRole("button", { name: labels["common.cancel"], exact: true }).click();

        await page.getByRole("button", { name: labels["common.sessions"], exact: true }).click();
        await page.getByRole("button", { name: labels["common.edit"], exact: true }).click();
        const editor = page.getByRole("dialog", { name: labels["editor.editProfile"] });
        await expectDialogFits(editor);
        const memo = editor.getByLabel(labels["common.optionalTask"]);
        await memo.scrollIntoViewIfNeeded();
        await expect(memo).toBeInViewport({ ratio: 1 });
        await expect.poll(() => editor.locator(".dialog-body").evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
        await expectDialogFits(editor);

        await editor.getByRole("button", { name: labels["editor.removeFromList"] }).click();
        const confirmation = page.getByRole("dialog", { name: labels["confirmation.removeSession"] });
        await expectDialogFits(confirmation);
        await confirmation.getByRole("button", { name: labels["common.cancel"], exact: true }).click();
        await editor.getByRole("button", { name: labels["common.cancel"], exact: true }).click();

        await page.getByRole("button", { name: labels["common.renew"], exact: true }).click();
        const renew = page.getByRole("dialog", { name: labels["confirmation.renewMfa"] });
        await expectDialogFits(renew);
        await renew.getByRole("button", { name: labels["common.cancel"], exact: true }).click();

        await page.getByRole("button", { name: labels["common.checkStatus"], exact: true }).click();
        const selected = page.getByRole("region", { name: labels["sessions.selected"] });
        await expect(selected.getByText("123456789012", { exact: true })).toBeVisible();

        await page.getByRole("button", { name: /Development SSO team-dev/ }).click();
        await page.getByRole("button", { name: labels["common.edit"], exact: true }).click();
        await expect(editor.getByRole("button", { name: labels["common.ssoWizard"] })).toBeInViewport({ ratio: 1 });
        await expectDialogFits(editor);
        await editor.getByRole("button", { name: labels["common.cancel"], exact: true }).click();

        await page.getByRole("button", { name: labels["share.export"], exact: true }).click();
        const share = page.getByRole("dialog", { name: labels["share.title"] });
        await expect(share.getByText(labels["share.credentialsExcluded"])).toBeInViewport({ ratio: 1 });
        await expectDialogFits(share);
        await share.getByRole("button", { name: labels["share.export"], exact: true }).click();
        await expect(share.getByRole("status")).toHaveText(labels["share.exported"]);
        await share.getByRole("button", { name: labels["common.close"], exact: true }).click();

        await page.getByRole("button", { name: labels["common.importShare"] }).click();
        const importPreview = page.getByRole("dialog", { name: labels["share.importTitle"] });
        await expectDialogFits(importPreview);
        const importBody = importPreview.locator(".dialog-body");
        await importBody.evaluate((element) => { element.scrollTop = element.scrollHeight; });
        await expect.poll(() => importBody.evaluate((element) => element.scrollTop)).toBeGreaterThan(0);
        await expectDialogFits(importPreview);
      });
    });
  }
}
