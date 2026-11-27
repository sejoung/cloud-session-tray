import { expect, test } from "@playwright/test";
import en from "../../locales/en.json" with { type: "json" };
import ko from "../../locales/ko.json" with { type: "json" };

test("switches the desktop language through settings and returns to the system language", async ({ page }) => {
  await page.goto("/tests/e2e/fixture.html");
  await page.getByRole("button", { name: ko["common.settings"], exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "ko");
  await page.getByRole("combobox", { name: ko["settings.language"] }).selectOption("en");
  await page.getByRole("button", { name: ko["common.save"], exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("button", { name: en["settings.logs"] })).toBeVisible();
  await expect(page.getByRole("button", { name: en["common.sessions"], exact: true })).toBeVisible();

  await page.getByRole("combobox", { name: en["settings.language"] }).selectOption("");
  await page.getByRole("button", { name: en["common.save"], exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "ko");
});
