import { defineConfig } from "@playwright/test";

const baseURL = "http://127.0.0.1:4176";

export default defineConfig({
  testDir: "./tests/e2e",
  testMatch: "**/*.e2e.ts",
  reporter: "list",
  use: {
    baseURL,
    browserName: "chromium",
    locale: "ko-KR",
    channel: process.env.CST_E2E_BROWSER_CHANNEL,
    viewport: { width: 1100, height: 760 },
    timezoneId: "Asia/Seoul",
    reducedMotion: "reduce",
  },
  projects: [
    { name: "chromium" },
    { name: "manual-ko", testMatch: "manual.e2e.ts" },
    { name: "manual-en", testMatch: "manual.e2e.ts", use: { locale: "en-US" } },
  ],
  webServer: {
    command: "npm run dev -- --port 4176",
    url: `${baseURL}/tests/e2e/fixture.html`,
    reuseExistingServer: !process.env.CI,
  },
});
