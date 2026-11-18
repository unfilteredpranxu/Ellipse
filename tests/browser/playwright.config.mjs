import {defineConfig} from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "**/*.spec.mjs",
  outputDir: "test-results",
  workers: 2,
  use: {
    baseURL: "http://127.0.0.1:4174",
    viewport: {width: 1536, height: 1024},
    locale: "en-GB",
    timezoneId: "Europe/London",
    reducedMotion: "reduce",
    launchOptions: process.env.CHROMIUM_PATH ? {
      executablePath: process.env.CHROMIUM_PATH,
      args: ["--no-sandbox", "--disable-dev-shm-usage"],
    } : {},
  },
  webServer: {
    command: "python3 -m http.server 4174 --bind 127.0.0.1 --directory ../../dist",
    url: "http://127.0.0.1:4174",
    reuseExistingServer: true,
  },
});
