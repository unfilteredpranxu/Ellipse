import {expect, test} from "@playwright/test";

const sdk = "https://www.gstatic.com/firebasejs/**/firebase-*.js";
const fakeApp = `
  export const getApps = () => [];
  export const initializeApp = () => ({});
`;
// Replace only the third-party Firebase SDK at the network boundary.
// The built frontend always uses the production Firebase bridge.
const fakeAuth = `
  const people = {
    primary: {uid:"uia", displayName:"Example Reader ".repeat(8), email:"reader@example.invalid"},
    second: {uid:"uib", displayName:"Another Reader", email:"other@example.invalid"},
  };
  const auth = {currentUser: people[sessionStorage.getItem("session")] ?? null};
  const listeners = new Set();
  function session(user) {
    auth.currentUser = user;
    if (user) sessionStorage.setItem("session", window.__testAccount || "primary");
    else sessionStorage.removeItem("session");
    for (const listener of listeners) listener(user);
  }
  export const getAuth = () => auth;
  export function onAuthStateChanged(_, listener) {
    listeners.add(listener);
    const timer = setTimeout(() => listener(auth.currentUser), window.__authDelay || 0);
    return () => { clearTimeout(timer); listeners.delete(listener); };
  }
  export class GoogleAuthProvider {
    addScope(scope) { this.scope = scope; }
    static credentialFromResult(result) { return result.credential; }
  }
  export async function signInWithPopup() { session(people[window.__testAccount || "primary"]); }
  export async function signOut() { session(null); }
  export async function reauthenticateWithPopup(user, provider) {
    if (user !== auth.currentUser || !provider.scope.endsWith("/drive.appdata")) throw Error("Bad Drive request");
    return {credential:{accessToken:"fixture-token"}};
  }
`;

async function stubFirebase(page, delay = 0) {
  await page.addInitScript(value => { window.__authDelay = value; }, delay);
  await page.route(sdk, route => route.fulfill({
    contentType: "application/javascript",
    headers: {"Access-Control-Allow-Origin": "*"},
    body: route.request().url().endsWith("/firebase-app.js") ? fakeApp : fakeAuth,
  }));
}

const homeMode = page => page.getByLabel("Home mode");
async function signIn(page) {
  await page.getByRole("button", {name: "Continue with Google"}).click();
  await expect(homeMode(page)).toBeVisible();
}

test("auth resolution, keyboard mode changes, session restoration and account isolation", async ({page}) => {
  await stubFirebase(page, 900);
  await page.goto("/");
  await expect(page.getByText("Checking your Google session…")).toBeVisible();
  await expect(homeMode(page)).toHaveCount(0);
  await signIn(page);
  await expect(homeMode(page)).toHaveValue("New Tab");
  await homeMode(page).focus();
  await expect(homeMode(page)).toBeFocused();
  await page.keyboard.press("ArrowDown");
  await expect(homeMode(page)).toHaveValue("Tablet");
  await expect(page.locator("main h1")).toHaveText("Your Space, Refined.");
  await page.reload();
  await expect(page.getByText("Checking your Google session…")).toBeVisible();
  await expect(homeMode(page)).toHaveCount(0);
  await expect(homeMode(page)).toHaveValue("Tablet");
  await page.getByRole("button", {name:"Google account", exact:true}).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByRole("button", {name:"Sign out", exact:true}).click();
  await expect(page.getByRole("button", {name:"Continue with Google"})).toBeVisible();
  await expect(homeMode(page)).toHaveCount(0);
  await page.evaluate(() => { window.__testAccount = "second"; });
  await signIn(page);
  await expect(homeMode(page)).toHaveValue("New Tab");
});

test("offline Firebase failure and native blocked state stay outside the workspace", async ({page}) => {
  await page.route(sdk, route => route.abort());
  await page.goto("/");
  await expect(page.getByText(/Could not load Google sign-in/)).toBeVisible();
  await expect(homeMode(page)).toHaveCount(0);
  await page.addInitScript(() => { window.__TAURI_INTERNALS__ = {}; });
  await page.reload();
  await page.getByRole("button", {name: "Continue with Google"}).click();
  await expect(page.getByText(/Native sign-in is not yet configured/)).toBeVisible();
  await expect(homeMode(page)).toHaveCount(0);
});

test("clock settings, live time, wallpaper, block layout, local and Drive presets", async ({page}) => {
  test.setTimeout(60000);
  await stubFirebase(page);
  let backup;
  const driveHeaders = {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Headers": "Authorization, Content-Type",
    "Access-Control-Allow-Methods": "GET, POST, PATCH, OPTIONS",
  };
  await page.route("https://www.googleapis.com/**", async route => {
    const url = new URL(route.request().url());
    const method = route.request().method();
    const respond = json => route.fulfill({json, headers: driveHeaders});
    if (method === "OPTIONS") return route.fulfill({status: 204, headers: driveHeaders});
    expect(route.request().headers().authorization).toBe("Bearer fixture-token");
    if (method === "POST") {
      const body = route.request().postData();
      backup = JSON.parse(body.split("Content-Type: application/json\r\n\r\n")[1].split("\r\n--")[0]);
      return respond({id: "fixture"});
    }
    if (method === "PATCH") {
      backup = route.request().postDataJSON();
      return respond({id: "fixture"});
    }
    if (url.searchParams.get("alt") === "media") return respond(backup);
    expect(url.searchParams.get("spaces")).toBe("appDataFolder");
    return respond({files: backup ? [{id: "fixture", size: String(JSON.stringify(backup).length)}] : []});
  });
  await page.clock.install({time: new Date("2026-11-18T21:41:23Z")});
  await page.goto("/");
  await signIn(page);
  const digits = page.locator("[data-block=clock] .clock-digits");
  await expect(digits).toHaveText("9:41");

  await page.getByRole("button", {name: "Customise clock", exact:true}).click();
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Time format").selectOption("24");
  await dialog.getByLabel("Seconds").check();
  await dialog.getByLabel("Show date").uncheck();
  await dialog.getByRole("button", {name:"Monospace", exact:true}).click();
  await dialog.getByLabel("Opacity").fill("78");
  await dialog.getByLabel("Blur").fill("36");
  await dialog.getByLabel("Horizontal").selectOption("end");
  await dialog.getByRole("slider", {name:"Size", exact:true}).fill("115");
  await dialog.getByLabel("Weight").fill("800");
  await expect(dialog.locator(".clock-digits")).toHaveText(/21:41:\d{2}/);
  await expect(dialog.locator(".clock-date")).toHaveCount(0);
  await expect(dialog.locator(".clock-face")).toHaveCSS("justify-items", "end");
  const before = await digits.textContent();
  await page.clock.fastForward(3000);
  await expect(digits).not.toHaveText(before);
  await page.keyboard.press("Escape");
  await expect(dialog).toHaveCount(0);
  await expect(page.getByRole("button", {name:"Customise clock", exact:true})).toBeFocused();
  await page.setViewportSize({width:320, height:568});
  expect(await digits.evaluate(node =>
    node.getBoundingClientRect().width <= node.closest(".clock-face").getBoundingClientRect().width,
  )).toBe(true);
  await page.setViewportSize({width:1536, height:1024});

  await page.getByRole("button", {name:"Settings", exact:true}).click();
  await dialog.getByLabel(/Upload wallpaper/).setInputFiles({
    name: "pixel.png", mimeType: "image/png",
    buffer: Buffer.from("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a7ioAAAAASUVORK5CYII=", "base64"),
  });
  await expect(dialog.getByRole("combobox", {name:"Wallpaper", exact:true})).toHaveValue("photo");
  await dialog.getByRole("combobox", {name:"Wallpaper", exact:true}).selectOption("artwork");
  await dialog.getByRole("checkbox", {name:"Palette", exact:true}).uncheck();
  await dialog.getByLabel("Hero artwork size").selectOption("wide");
  await dialog.getByRole("button", {name:"Move Hero artwork up", exact:true}).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".workspace-wallpaper")).toHaveAttribute("src", /ellipse-knight-clean/);
  await expect(page.locator("[data-block=palette]")).toHaveCount(0);
  await expect(page.locator(".bento > div:first-child")).toHaveAttribute("data-block", "hero");
  await expect(page.locator("[data-block=hero]")).toHaveClass(/wide/);

  await page.getByRole("button", {name:"Presets", exact:true}).click();
  await dialog.getByLabel("Preset name").fill("Evening");
  await dialog.getByRole("button", {name:"Save preset"}).click();
  await dialog.getByRole("button", {name:"Back up presets"}).click();
  await dialog.getByRole("button", {name:"Continue with Google"}).click();
  await expect(dialog.getByText("Presets backed up to Google Drive.")).toBeVisible();
  expect(backup[0].name).toBe("Evening");
  expect(backup[0].settings.appearance.clock.opacity).toBe(78);
  await dialog.getByRole("button", {name:"Back up presets"}).click();
  await dialog.getByRole("button", {name:"Continue with Google"}).click();
  await expect(dialog.getByText("Presets backed up to Google Drive.")).toBeVisible();
  await dialog.getByRole("button", {name:"Remove", exact:true}).click();
  await dialog.getByRole("button", {name:"Restore from Drive"}).click();
  await dialog.getByRole("button", {name:"Continue with Google"}).click();
  await expect(dialog.getByText("Presets restored from Google Drive.")).toBeVisible();
  await page.keyboard.press("Escape");
  await page.reload();
  await expect(digits).toHaveAttribute("data-font", "mono");
  await expect(page.locator("[data-block=palette]")).toHaveCount(0);

  await page.getByRole("button", {name:"Customise clock", exact:true}).click();
  await dialog.getByRole("button", {name:"Reset clock"}).click();
  await expect(dialog.getByLabel("Time format")).toHaveValue("12");
  await expect(dialog.getByLabel("Opacity")).toHaveValue("96");
  await expect(dialog.getByLabel("Show date")).toBeChecked();
  await page.keyboard.press("Escape");
  await page.getByRole("button", {name:"Presets", exact:true}).click();
  await dialog.getByRole("button", {name:"Apply", exact:true}).click();
  await expect(digits).toHaveAttribute("data-font", "mono");
});

test("three responsive layouts, image sizing, touch targets, and screenshot evidence", async ({page}, info) => {
  await stubFirebase(page);
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.clock.setFixedTime(new Date("2026-11-18T09:41:23Z"));
  await page.goto("/");
  await signIn(page);
  await page.evaluate(() => document.fonts.ready);
  const viewports = [[320,568],[360,640],[375,812],[390,844],[430,932],[768,1024],
    [820,1180],[1024,768],[1280,800],[1440,900],[1536,1024],[844,390],[667,375]];
  for (const [width, height] of viewports) {
    await page.setViewportSize({width, height});
    for (const mode of ["New Tab", "Tablet", "Magazine"]) {
      await homeMode(page).selectOption(mode);
      await expect(page.locator("main h1")).toHaveCount(1);
      if (mode === "Tablet" && width >= 1024) {
        expect(await page.locator("main h1").evaluate(heading => {
          const text = document.createRange();
          text.selectNodeContents(heading);
          return text.getClientRects().length;
        })).toBe(1);
      }
      const issues = await page.evaluate(() => {
        const screen = document.documentElement.clientWidth;
        const rootOverflow = document.documentElement.scrollWidth > screen;
        const failed = [...document.querySelectorAll("main h1:not(.sr-only), .clock-digits, .art-tile, .workspace-nav button")]
          .filter(node => {
            const bounds = node.getBoundingClientRect();
            return bounds.right > screen + 1 || bounds.left < -1 || (node.classList.contains("clock-digits") &&
              bounds.width > node.closest(".clock-face").getBoundingClientRect().width + 1);
          }).map(node => node.className || node.textContent);
        return {rootOverflow, failed};
      });
      expect(issues, `${mode} at ${width}×${height}`).toEqual({rootOverflow:false, failed:[]});
      const image = page.locator(".art-tile img").first();
      await expect.poll(() => image.evaluate(img => img.complete && img.naturalWidth === 3344)).toBe(true);
      for (const button of await page.locator(".workspace-nav button, .workspace main .art-open").all()) {
        const rect = await button.boundingBox();
        expect(rect.height).toBeGreaterThanOrEqual(44);
      }
      if ([390, 820, 1536].includes(width)) {
        await page.screenshot({path: info.outputPath(`${mode.toLowerCase().replace(" ", "-")}-${width}.png`), fullPage:true});
      }
    }
  }
  expect(errors).toEqual([]);
});
