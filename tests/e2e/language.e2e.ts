import { expect, test } from "@playwright/test";
import en from "../../locales/en.json" with { type: "json" };
import ko from "../../locales/ko.json" with { type: "json" };

test("saves the display language and expiration notifications from the settings header", async ({ page }) => {
  await page.goto("/tests/e2e/fixture.html");
  await page.getByRole("button", { name: ko["common.settings"], exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "ko");
  const settingsHeader = page.locator("main > #preferences > header");
  await page.getByRole("combobox", { name: ko["settings.language"] }).selectOption("en");
  await page.getByRole("spinbutton", { name: ko["settings.notifyMinutes"] }).fill("25");
  await settingsHeader.getByRole("button", { name: ko["settings.save"], exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("button", { name: en["settings.logs"] })).toBeVisible();
  await page.getByRole("button", { name: en["common.sessions"], exact: true }).click();
  await page.getByRole("button", { name: en["common.settings"], exact: true }).click();
  await expect(page.getByRole("spinbutton", { name: en["settings.notifyMinutes"] })).toHaveValue("25");

  await page.getByRole("combobox", { name: en["settings.language"] }).selectOption("");
  await settingsHeader.getByRole("button", { name: en["settings.save"], exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "ko");
});
