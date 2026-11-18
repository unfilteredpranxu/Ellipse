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
  await dialog.getByRole("button", {name:"Edit wallpaper", exact:true}).click();
  await dialog.getByRole("button", {name:"Use Ellipse artwork", exact:true}).click();
  await dialog.getByRole("button", {name:"Apply image", exact:true}).click();
  await page.getByRole("button", {name:"Settings", exact:true}).click();
  await dialog.getByRole("checkbox", {name:"Palette", exact:true}).uncheck();
  await dialog.getByLabel("Hero artwork size").selectOption("wide");
  await dialog.getByRole("button", {name:"Move Hero artwork up", exact:true}).click();
  await page.keyboard.press("Escape");
  await expect(page.locator(".workspace-wallpaper img")).toHaveAttribute("src", /ellipse-knight-clean/);
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

async function picture(page, name, hues, size = [160, 80]) {
  const base64 = await page.evaluate(({colors, size}) => {
    const canvas = document.createElement("canvas");
    [canvas.width, canvas.height] = size;
    const ctx = canvas.getContext("2d");
    colors.forEach((color, i) => {
      ctx.fillStyle = color;
      ctx.fillRect(i * canvas.width / 4, 0, canvas.width / 4, canvas.height);
    });
    return canvas.toDataURL().split(",")[1];
  }, {colors:hues,size});
  return {name, mimeType:"image/png", buffer:Buffer.from(base64,"base64")};
}

const imageBlock = (page, slot) => page.locator(`[data-artwork="${slot}"] img`);
const editBlock = (page, slot) => page.getByRole("button", {name:`Edit ${slot} artwork`, exact:true}).click();

async function applyImage(page, file) {
  const dialog = page.getByRole("dialog");
  await dialog.getByLabel("Choose local image").setInputFiles(file);
  await expect(dialog.getByRole("img", {name:"Artwork preview"})).toHaveAttribute("src", /^blob:/);
  await dialog.getByRole("button", {name:"Apply image", exact:true}).click();
  await expect(dialog).toHaveCount(0);
}

test("per-block image persistence, reuse, cancel, unsupported files, quota and explicit restore", async ({page}) => {
  test.setTimeout(75000);
  await stubFirebase(page);
  const errors = [];
  page.on("pageerror", error => errors.push(error.message));
  await page.goto("/");
  await signIn(page);
  const original = /ellipse-knight-clean\.jpg$/;
  const red = await picture(page, "red.png", ["#a84929", "#b85529", "#27558e"]);
  const blue = await picture(page, "blue.png", ["#18478d", "#8b8fc6"], [3000, 900]);
  const dialog = page.getByRole("dialog");

  await editBlock(page, "New Tab hero");
  await dialog.getByLabel("Choose local image").setInputFiles({name:"unsupported.svg",mimeType:"image/svg+xml",buffer:Buffer.from("<svg/>")});
  await expect(dialog.getByText(/Choose a JPEG, PNG, WebP/)).toBeVisible();
  await dialog.getByLabel("Choose local image").setInputFiles({name:"oversized.png",mimeType:"image/png",buffer:Buffer.alloc(24 * 1024 * 1024 + 1)});
  await expect(dialog.getByText(/no larger than 24 MiB/)).toBeVisible();
  const transfer = await page.evaluateHandle(bytes => {
    const transfer = new DataTransfer();
    transfer.items.add(new File([Uint8Array.from(bytes)], "red.png", {type:"image/png"}));
    return transfer;
  }, [...red.buffer]);
  await dialog.locator(".image-editor").dispatchEvent("drop", {dataTransfer: transfer});
  await transfer.dispose();
  await expect(dialog.locator(".image-preview img")).toHaveAttribute("src", /^blob:/);
  await dialog.getByRole("button", {name:"Apply image"}).click();
  await expect(dialog).toHaveCount(0);
  const hero = imageBlock(page, "New Tab hero");
  const detail = imageBlock(page, "New Tab detail");
  await expect(hero).toHaveAttribute("src", /^blob:/);
  await expect(detail).toHaveAttribute("src", original);

  await editBlock(page, "New Tab detail");
  await dialog.getByLabel("Choose local image").setInputFiles(blue);
  await expect(dialog.locator(".image-preview img")).toHaveAttribute("src", /^blob:/);
  await dialog.getByRole("slider", {name:"Horizontal position"}).fill("73");
  await dialog.getByRole("slider", {name:"Zoom"}).fill("160");
  await dialog.getByRole("combobox", {name:"Fit"}).selectOption("contain");
  await expect(dialog.locator(".image-preview img")).toHaveCSS("object-position", "73% 50%");
  await dialog.getByRole("button", {name:"Apply image"}).click();
  await expect(detail).toHaveAttribute("src", /^blob:/);
  await expect(detail).toHaveCSS("object-fit", "contain");
  await expect.poll(() => detail.evaluate(image => image.naturalWidth)).toBe(2560);
  expect(await hero.getAttribute("src")).not.toBe(await detail.getAttribute("src"));
  await page.reload();
  await expect(hero).toHaveAttribute("src", /^blob:/);
  await expect(detail).toHaveCSS("object-position", "73% 50%");
  const previous = await hero.getAttribute("src");
  await editBlock(page, "New Tab hero");
  await dialog.getByRole("button", {name:"Restore original"}).click();
  await page.keyboard.press("Escape");
  await expect(hero).toHaveAttribute("src", previous);
  await expect(page.getByRole("button", {name:"Edit New Tab hero artwork"})).toBeFocused();

  await editBlock(page, "New Tab hero");
  await dialog.getByLabel("Choose local image").setInputFiles(blue);
  await expect(dialog.locator(".image-preview img")).toHaveAttribute("src", /^blob:/);
  await page.evaluate(() => {
    window.__originalPut = IDBObjectStore.prototype.put;
    IDBObjectStore.prototype.put = () => { throw new DOMException("fixture quota", "QuotaExceededError"); };
  });
  await dialog.getByRole("button", {name:"Apply image"}).click();
  await expect(dialog.getByText(/storage is full/)).toBeVisible();
  await expect(hero).toHaveAttribute("src", previous);
  await page.evaluate(() => { IDBObjectStore.prototype.put = window.__originalPut; });
  await dialog.getByRole("button", {name:"Cancel", exact:true}).click();

  await homeMode(page).selectOption("Tablet");
  await expect(imageBlock(page, "Tablet feature")).toHaveAttribute("src", original);
  await editBlock(page, "Tablet feature");
  await dialog.getByRole("combobox", {name:"Reuse assigned image"}).selectOption({label:"New Tab hero"});
  await expect(dialog.locator(".image-preview img")).toHaveAttribute("src", /^blob:/);
  await dialog.getByRole("button", {name:"Apply image"}).click();
  await expect(imageBlock(page, "Tablet feature")).toHaveAttribute("src", /^blob:/);
  await expect(imageBlock(page, "Tablet detail")).toHaveAttribute("src", original);
  await homeMode(page).selectOption("Magazine");
  await expect(imageBlock(page, "Magazine feature")).toHaveAttribute("src", original);
  await editBlock(page, "Magazine presets thumbnail");
  await applyImage(page, blue);

  // Two unique files despite assigning them to four blocks.
  const stored = await page.evaluate(() => new Promise((resolve, reject) => {
    const request = indexedDB.open("ellipse-local-artwork", 1);
    request.onerror = reject;
    request.onsuccess = () => {
      const db = request.result;
      const list = db.transaction("images").objectStore("images").getAll();
      list.onsuccess = () => { db.close(); resolve(list.result.map(record => ({original:record.original instanceof Blob, preview:record.preview instanceof Blob}))); };
    };
  }));
  expect(stored.every(record => record.original)).toBe(true);
  expect(stored.filter(record => record.preview)).toHaveLength(1);
  expect(stored).toHaveLength(2);

  await page.getByRole("button", {name:"Google account", exact:true}).click();
  await dialog.getByRole("button", {name:"Sign out", exact:true}).click();
  await page.evaluate(() => { window.__testAccount = "second"; });
  await signIn(page);
  await expect(hero).toHaveAttribute("src", original);
  await page.getByRole("button", {name:"Google account", exact:true}).click();
  await dialog.getByRole("button", {name:"Sign out", exact:true}).click();
  await page.evaluate(() => { window.__testAccount = "primary"; });
  await signIn(page);
  await expect(imageBlock(page, "Magazine presets thumbnail")).toHaveAttribute("src", /^blob:/);

  await editBlock(page, "Magazine presets thumbnail");
  await dialog.getByRole("button", {name:"Restore original"}).click();
  await dialog.getByRole("button", {name:"Apply image"}).click();
  await expect(imageBlock(page, "Magazine presets thumbnail")).toHaveAttribute("src", original);
  await expect(dialog).toHaveCount(0);
  await page.getByRole("navigation", {name:"Workspace",exact:true}).getByRole("button", {name:"Artwork", exact:true}).click();
  await expect(dialog).toHaveAccessibleName("Workspace artwork");
  await dialog.getByRole("button", {name:"Restore all default artwork"}).click();
  await dialog.getByRole("button", {name:"Cancel reset"}).click();
  await dialog.getByRole("button", {name:"Restore all default artwork"}).click();
  await dialog.getByRole("button", {name:"Restore all",exact:true}).click();
  await expect(dialog.getByText("Default block artwork restored.")).toBeVisible();
  await page.keyboard.press("Escape");
  await homeMode(page).selectOption("New Tab");
  await expect(hero).toHaveAttribute("src", original);
  await expect(detail).toHaveAttribute("src", original);
  expect(errors).toEqual([]);
});

test("wallpaper and Tablet palette persist, preview and cancel, and remain isolated from both dark modes", async ({page}, info) => {
  test.setTimeout(75000);
  await stubFirebase(page);
  await page.goto("/");
  await signIn(page);
  const stage = page.locator(".workspace");
  const dark = await stage.evaluate(node => getComputedStyle(node).getPropertyValue("--canvas").trim());
  await homeMode(page).selectOption("Tablet");
  const light = await stage.evaluate(node => getComputedStyle(node).getPropertyValue("--canvas").trim());
  const wallpaper = await picture(page, "sunset.png", ["#ae3a25", "#ad3b2a", "#2d6380", "#e5b642"]);
  const dialog = page.getByRole("dialog");
  await page.getByRole("button", {name:"Settings", exact:true}).click();
  await dialog.getByRole("button", {name:"Edit wallpaper"}).click();
  await applyImage(page, wallpaper);
  await expect(page.locator(".workspace-wallpaper img")).toHaveAttribute("src", /^blob:/);
  await expect(stage).toHaveClass(/workspace-themed/);
  await expect(imageBlock(page, "Tablet feature")).toHaveAttribute("src", /ellipse-knight-clean/);

  await page.getByRole("button", {name:"Palette", exact:true}).click();
  await expect(dialog.locator(".theme-swatch")).toHaveCount(5);
  await page.context().grantPermissions(["clipboard-read", "clipboard-write"]);
  const swatch = dialog.locator(".theme-swatch").first();
  const hex = await swatch.locator("code").innerText();
  await swatch.click();
  await expect(dialog.getByText("Color copied.")).toBeVisible();
  expect(await page.evaluate(() => navigator.clipboard.readText())).toBe(hex);
  await dialog.getByRole("button", {name:"Regenerate palette"}).click();
  await expect(dialog.getByRole("button", {name:"Regenerate palette"})).toBeEnabled();
  await dialog.getByRole("button", {name:"Apply palette"}).click();
  const themed = await stage.evaluate(node => getComputedStyle(node).getPropertyValue("--canvas").trim());
  expect(themed).not.toBe(light);
  for (const mode of ["New Tab", "Magazine"]) {
    await homeMode(page).selectOption(mode);
    await expect(stage).not.toHaveClass(/workspace-themed/);
    expect(await stage.evaluate(node => getComputedStyle(node).getPropertyValue("--canvas").trim())).toBe(dark);
  }
  await homeMode(page).selectOption("Tablet");
  await page.reload();
  await expect(stage).toHaveClass(/workspace-themed/);
  expect(await stage.evaluate(node => getComputedStyle(node).getPropertyValue("--canvas").trim())).toBe(themed);

  await page.getByRole("button", {name:"Palette", exact:true}).click();
  await dialog.getByRole("button", {name:"Reset Tablet theme"}).click();
  await expect(stage).not.toHaveClass(/workspace-themed/);
  await page.setViewportSize({width:320,height:568});
  await page.getByRole("button", {name:"Palette", exact:true}).click();
  await dialog.getByRole("button", {name:"Regenerate palette"}).click();
  await expect(dialog.locator(".theme-swatch")).toHaveCount(5);
  await dialog.getByRole("button", {name:"Preview palette"}).click();
  await expect(stage).toHaveClass(/workspace-themed/);
  const layout = await dialog.evaluate(node => ({
    fits: node.scrollWidth <= node.clientWidth + 1,
    viewport: node.getBoundingClientRect().right <= innerWidth,
  }));
  expect(layout).toEqual({fits:true,viewport:true});
  await page.screenshot({path:info.outputPath("tablet-palette-320.png")});
  await dialog.getByRole("button", {name:"Cancel", exact:true}).click();
  await expect(stage).not.toHaveClass(/workspace-themed/);
  await editBlock(page, "Tablet feature");
  await expect(dialog).toBeVisible();
  await page.screenshot({path:info.outputPath("artwork-editor-320.png")});
  await page.setViewportSize({width:844,height:390});
  expect(await dialog.evaluate(node => node.getBoundingClientRect().bottom <= innerHeight)).toBe(true);
  await page.screenshot({path:info.outputPath("artwork-editor-landscape.png")});
  await dialog.getByRole("button", {name:"Cancel", exact:true}).click();
  await expect(dialog).toHaveCount(0);
});

test("legacy wallpaper and clock settings still load from version one", async ({page}) => {
  await stubFirebase(page);
  await page.goto("/");
  const image = await picture(page, "legacy.png", ["#6a7b96"]);
  await page.evaluate(data => localStorage.setItem("ellipse:uia:appearance:v1",
    JSON.stringify({mode:"Magazine", appearance:{wallpaper:{Photo:data}, wallpaper_opacity:29, clock:{twenty_four:true}}})),
  `data:image/png;base64,${image.buffer.toString("base64")}`);
  await signIn(page);
  await expect(homeMode(page)).toHaveValue("Magazine");
  await expect(page.locator(".workspace-wallpaper img")).toHaveAttribute("src", /^data:image\/png/);
  await homeMode(page).selectOption("New Tab");
  await page.reload();
  await expect(homeMode(page)).toHaveValue("New Tab");
  await expect(page.locator(".workspace-wallpaper img")).toHaveAttribute("src", /^data:image\/png/);
  await expect(page.locator("[data-block=clock] .clock-period")).toHaveCount(0);
});
