import { defineConfig } from "@playwright/test";
import tauri from "./src-tauri/tauri.conf.json" with { type: "json" };

const baseURL = "http://127.0.0.1:4176";
const [{ width, height }] = tauri.app.windows;
const channel = process.env.CST_E2E_BROWSER_CHANNEL;

export default defineConfig({
  testDir: "./tests/e2e",
  testMatch: "**/*.e2e.ts",
  reporter: "list",
  use: {
    baseURL,
    browserName: "chromium",
    locale: "ko-KR",
    viewport: { width, height },
    timezoneId: "Asia/Seoul",
    reducedMotion: "reduce",
  },
  projects: [
    { name: "chromium", use: { channel } },
    { name: "webkit-layout", testMatch: "layout.e2e.ts", use: { browserName: "webkit" } },
    { name: "manual-ko", testMatch: "manual.e2e.ts", use: { channel } },
    { name: "manual-en", testMatch: "manual.e2e.ts", use: { channel, locale: "en-US" } },
  ],
  webServer: {
    command: "npm run dev -- --port 4176",
    url: `${baseURL}/tests/e2e/fixture.html`,
    reuseExistingServer: !process.env.CI,
  },
});
