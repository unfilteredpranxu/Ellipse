import {defineConfig} from "@playwright/test";

const port = Number(process.env.ELLIPSE_BROWSER_PORT || 4174);
const origin = `http://127.0.0.1:${port}`;

export default defineConfig({
  testDir: ".",
  testMatch: "**/*.spec.mjs",
  outputDir: "test-results",
  workers: 2,
  use: {
    baseURL: origin,
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
    command: `python3 -m http.server ${port} --bind 127.0.0.1 --directory ../../dist`,
    url: origin,
    reuseExistingServer: true,
  },
});
