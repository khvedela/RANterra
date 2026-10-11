import { defineConfig } from "@playwright/test";
const hostedOrigin = process.env.RANTERRA_TEST_ORIGIN;
export default defineConfig({
  testDir: "./tests",
  timeout: 30000,
  workers: 1,
  use: {
    headless: process.env.RANTERRA_HEADED !== "1",
    screenshot: "only-on-failure",
    baseURL: hostedOrigin ?? "http://127.0.0.1:4173",
    viewport: { width: 1600, height: 1100 },
    trace: "retain-on-failure",
  },
  webServer: hostedOrigin
    ? undefined
    : {
        command: "node scripts/serve-dist.mjs",
        url: "http://127.0.0.1:4173",
        reuseExistingServer: false,
      },
  projects: [
    { name: "chromium", use: { browserName: "chromium" } },
    { name: "firefox", use: { browserName: "firefox" } },
  ],
});
