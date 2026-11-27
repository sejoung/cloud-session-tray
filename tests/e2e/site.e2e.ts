import { expect, test } from "@playwright/test";

const releaseApi = "https://api.github.com/repos/sejoung/cloud-session-tray/releases/latest";
const releasePage = "https://github.com/sejoung/cloud-session-tray/releases/latest";
const assets = ["Cloud.Session.Tray_0.1.3_aarch64.dmg", "Cloud.Session.Tray_0.1.3_x64-setup.exe"].map((name) => ({
  name, browser_download_url: `https://github.com/sejoung/cloud-session-tray/releases/download/v0.1.3/${name}`,
}));

test.use({ locale: "fr-FR" });

test("links both languages to the matching latest installer on desktop and mobile", async ({ page }) => {
  await page.route(releaseApi, (route) => route.fulfill({ json: { assets } }));
  await page.goto("/docs/");
  await expect(page.locator("html")).toHaveAttribute("lang", "en");
  await expect(page.getByRole("link", { name: /Download for macOS/ })).toHaveAttribute("href", assets[0].browser_download_url);
  await expect(page.getByRole("link", { name: /Download for Windows/ })).toHaveAttribute("href", assets[1].browser_download_url);
  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("link", { name: "한국어" }).click();
  await expect(page.locator("html")).toHaveAttribute("lang", "ko");
  await expect(page.getByRole("link", { name: /macOS 다운로드/ })).toHaveAttribute("href", assets[0].browser_download_url);
  await expect(page.getByRole("link", { name: /Windows 다운로드/ })).toHaveAttribute("href", assets[1].browser_download_url);
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBe(390);
  await expect(page.locator("header nav a")).toHaveCount(3);
  await page.getByRole("link", { name: "IAM MFA", exact: true }).click();
  await expect(page).toHaveURL(/\/ko\/#iam$/);
  await expect(page.getByRole("heading", { name: "IAM 사용자와 MFA 연결" })).toBeInViewport();
});

test.describe("Korean browser", () => {
  test.use({ locale: "ko-KR" });

  test("opens Korean by default and respects an explicit English choice", async ({ page }) => {
    await page.route(releaseApi, (route) => route.abort());
    await page.goto("/docs/?ref=readme#iam");
    await expect(page).toHaveURL(/\/docs\/ko\/\?ref=readme#iam$/);
    await expect(page.locator("html")).toHaveAttribute("lang", "ko");

    await page.getByRole("link", { name: "English", exact: true }).click();
    await expect(page).toHaveURL(/\/docs\/\?lang=en$/);
    await expect(page.locator("html")).toHaveAttribute("lang", "en");
  });
});

test("falls back to the latest release page when the asset lookup is unavailable", async ({ page }) => {
  await page.route(releaseApi, (route) => route.fulfill({ status: 503, body: "Unavailable" }));
  const request = page.waitForResponse(releaseApi);
  await page.goto("/docs/");
  await request;
  for (const platform of ["macOS", "Windows"]) {
    await expect(page.getByRole("link", { name: new RegExp(`Download for ${platform}`) })).toHaveAttribute("href", releasePage);
  }
});
