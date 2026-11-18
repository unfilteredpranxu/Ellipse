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

async function notesPdf() {
  const {PDFDocument, StandardFonts, degrees} = await import("../../web/pdf-runtime/pdf-lib.mjs");
  const pdf=await PDFDocument.create();
  const font=await pdf.embedFont(StandardFonts.Helvetica);
  for (const [index,text] of ["Ellipse field notes","Second page evidence"].entries()) {
    const page=pdf.addPage([420,530]);
    if (index===1) page.setRotation(degrees(90));
    page.drawText(text,{x:48,y:440,size:22,font});
  }
  return Buffer.from(await pdf.save());
}

const notesLibrary = page => page.getByRole("main",{name:"Notes library"});
const notesEditor = page => page.getByRole("main",{name:"Document editor"});

test("authenticated web Notes only reads explicitly opened local PDFs", async ({page}, info) => {
  await stubFirebase(page);
  await page.goto("/");
  await signIn(page);
  await page.getByRole("button",{name:"Notes",exact:true}).click();
  await expect(notesLibrary(page)).toBeVisible();
  await expect(page.getByRole("button",{name:"New notebook"})).toHaveCount(0);
  await expect(notesLibrary(page).getByText(/installed library stays/)).toBeVisible();
  const bytes=await notesPdf();
  await page.getByLabel("Choose PDF", {exact:true}).setInputFiles({name:"Field notes.pdf",mimeType:"application/pdf",buffer:bytes});
  await expect(notesEditor(page)).toBeVisible();
  await expect(notesLibrary(page)).toHaveCount(0);
  await expect(page.getByLabel("Writing tools")).toHaveCount(0);
  await expect(page.getByRole("button",{name:"Timer",exact:true})).toHaveCount(0);
  await expect.poll(() => page.locator(".notes-pdf").evaluate(canvas=>canvas.width)).toBeGreaterThan(300);
  await page.getByRole("button",{name:"Next page",exact:true}).click();
  await expect(page.getByLabel("Current page")).toHaveText("2 / 2");
  await expect(page.getByRole("button",{name:"Next page",exact:true})).toBeDisabled();
  await page.getByRole("button",{name:"Previous page",exact:true}).click();
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.getByRole("searchbox",{name:"Find in document"}).fill("Second page evidence");
  await page.getByRole("button",{name:"Find",exact:true}).click();
  await page.getByRole("button",{name:/2 · Second page evidence/}).click();
  await expect(page.getByLabel("Current page")).toHaveText("2 / 2");
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.screenshot({path:info.outputPath("notes-web-pdf.png")});
  const [download]=await Promise.all([page.waitForEvent("download"),page.getByRole("button",{name:"Download original"}).click()]);
  const {readFile}=await import("node:fs/promises");
  expect(await readFile(await download.path())).toEqual(bytes);
  await page.getByRole("button",{name:"Library",exact:false}).click();
  await expect(notesLibrary(page)).toBeVisible();
  await expect(page.getByRole("button",{name:"Field notes",exact:true})).toHaveAttribute("aria-pressed","true");
  await page.reload();
  await page.getByRole("button",{name:"Notes",exact:true}).click();
  await expect(page.getByRole("button",{name:"Field notes",exact:true})).toHaveCount(0);
});

async function stubNotesBridge(page, fixtures = false, recover = false) {
  await page.addInitScript(({seed,recover}) => {
    const colors=[[109,51,42],[89,79,64],[42,66,91],[148,116,77],[42,65,56],[97,64,75],[40,62,76]];
    function initial() {
      let next=1;
      const books=seed ? ["Physics","Calculus","Chemistry","Fieldwork","Literature","History","Mathematics","Observations","Philosophy","Economics","Mapping","Mechanics","Archive journal"].map((title,index)=>({
        id:next++,title,subject:"",collection:"",tags:[],color:colors[index%colors.length],cover:index%3===0?"Orbit":index%3===1?"Bands":"Cloth",
        folder:null,favorite:false,disposition:"Active",created:index+1,modified:index+1,opened:0,page:0,zoom:100,scroll:0,
        pen:{nib:"Ballpoint",width:3,opacity:1,color:[33,40,57],smoothing:0.23},presets:[],pdf:null,
        pages:[{id:next++,paper:{template:"Ruled",width:612,height:792,background:[255,252,245],pattern:[189,184,174],spacing:24},ink:[],text:[],bookmarked:false}],
      })) : [];
      return {schema:1,next,books,folders:[],study:{active:null,history:[]},dark:false};
    }
    window.__TAURI_INTERNALS__={invoke:async (command,args)=>{
      const key="qa-notes";
      const previous=JSON.parse(sessionStorage.getItem(key)||"null") || {head:recover?7:0,vault:initial(),recovered:recover};
      if (command==="notes_load") return previous;
      if (command==="notes_backup") {
        if (args.head!==previous.head) throw Error("Revision conflict.");
        return "Documents/Ellipse Backups/Ellipse Notes 1.ellipsevault";
      }
      if (command==="notes_restore") {
        if (args.head!==previous.head) throw Error("Revision conflict.");
        let vault;
        try {
          vault=JSON.parse(atob(args.encoded));
        } catch {
          throw Error("Invalid fixture backup.");
        }
        const snapshot={head:previous.head+1,vault,recovered:false};
        sessionStorage.setItem(key,JSON.stringify(snapshot));
        return snapshot;
      }
      if (command==="notes_save") {
        if (window.__failNotesSave) throw Error("Fixture storage unavailable.");
        if (args.head!==previous.head) throw Error("Revision conflict.");
        if (previous.recovered || window.__holdNextNotesSave) {
          window.__holdNextNotesSave=false;
          await new Promise(resolve=>{window.__finishNotesSave=resolve;});
        }
        await new Promise(resolve=>setTimeout(resolve,35));
        const head=args.head+1;
        sessionStorage.setItem(key,JSON.stringify({head,vault:args.vault,recovered:false}));
        return head;
      }
      if (command==="notes_store_pdf") {
        const id="b".repeat(64);
        sessionStorage.setItem("qa-pdf",args.encoded);
        return id;
      }
      if (command==="store_artwork") {
        sessionStorage.setItem(`qa-art:${args.account}:${args.key}`,args.encoded);
        return null;
      }
      if (command==="read_artwork") {
        const encoded=sessionStorage.getItem(`qa-art:${args.account}:${args.key}`);
        if (!encoded) throw Error("Fixture artwork missing.");
        return Uint8Array.from(atob(encoded),b=>b.charCodeAt(0));
      }
      if (command==="notes_read_pdf") {
        const encoded=sessionStorage.getItem("qa-pdf");
        if (!encoded) throw Error("Fixture PDF missing.");
        return Uint8Array.from(atob(encoded),b=>b.charCodeAt(0));
      }
      throw Error("Unknown test bridge command.");
    }};
  },{seed:fixtures,recover});
}

async function openLocalNotes(page) {
  await page.getByRole("button",{name:"Open Notes on this device"}).click();
  await expect(notesLibrary(page)).toBeVisible();
}

async function stroke(page, positions) {
  const area=page.getByLabel("Writing surface").boundingBox();
  const box=await area;
  const mapped=positions.map(([x,y])=>[box.x+box.width*x,box.y+box.height*y]);
  await page.mouse.move(...mapped[0]);
  await page.mouse.down();
  for (const point of mapped.slice(1)) await page.mouse.move(...point,{steps:6});
  await page.mouse.up();
}

async function notesSnapshot(page) {
  // The empty vault returned by notes_load has no committed revision yet.
  const snapshot=await page.waitForFunction(()=>sessionStorage.getItem("qa-notes"));
  try {
    return JSON.parse(await snapshot.jsonValue()).vault;
  } finally {
    await snapshot.dispose();
  }
}

test("Notes native bridge flow creates and saves vector pages, erases, recovers a failed save and logs study", async ({page},info) => {
  test.setTimeout(90000);
  await stubNotesBridge(page);
  await page.goto("/");
  await openLocalNotes(page);
  await page.getByRole("button",{name:"New notebook",exact:true}).click();
  await page.getByLabel("Name",{exact:true}).fill("Seminar");
  await page.getByLabel("Subject",{exact:true}).fill("Physics");
  await page.getByRole("button",{name:"Create notebook"}).click();
  await page.getByRole("button",{name:"Open",exact:true}).click();
  await expect(notesLibrary(page)).toHaveCount(0);
  await expect(notesEditor(page).getByRole("heading",{name:"Seminar"})).toBeVisible();
  await expect(page.getByRole("complementary",{name:"Study timer"})).toHaveCount(0);
  await expect(page.getByLabel("Writing surface")).toBeVisible();
  await stroke(page,[[.15,.25],[.80,.25]]);
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].ink.length).toBe(1);
  await page.getByLabel("Tool",{exact:true}).selectOption("Partial eraser");
  const canvas=page.getByLabel("Writing surface");
  const box=await canvas.boundingBox();
  await page.mouse.move(box.x+box.width*.45,box.y+box.height*.21);
  await page.mouse.down();
  await page.mouse.move(box.x+box.width*.45,box.y+box.height*.31,{steps:6});
  await canvas.dispatchEvent("pointercancel",{pointerId:1,pointerType:"mouse"});
  await page.mouse.up();
  expect((await notesSnapshot(page)).books[0].pages[0].ink).toHaveLength(1);
  await stroke(page,[[.45,.21],[.45,.31]]);
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].ink.length).toBe(2);
  await page.getByRole("button",{name:"Undo",exact:true}).click();
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].ink.length).toBe(1);
  await page.getByRole("button",{name:"Redo",exact:true}).click();
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].ink.length).toBe(2);
  await page.evaluate(()=> {window.__failNotesSave=true;});
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.getByRole("button",{name:"Insert after",exact:true}).click();
  await expect(page.getByText("Fixture storage unavailable.")).toBeVisible();
  await expect(page.getByLabel("Current page")).toHaveText("2 / 2");
  expect((await notesSnapshot(page)).books[0].pages).toHaveLength(1);
  await page.evaluate(()=> {window.__failNotesSave=false;});
  await page.getByRole("button",{name:"Retry save"}).click();
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages.length).toBe(2);
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.getByRole("button",{name:"Timer",exact:true}).click();
  await page.getByRole("button",{name:"Start timer"}).click();
  await expect(page.getByRole("button",{name:"Finish and log"})).toBeVisible();
  await page.getByRole("button",{name:"Timer",exact:true}).click();
  await expect(page.getByRole("complementary",{name:"Study timer"})).toHaveCount(0);
  await page.getByRole("button",{name:"Timer",exact:true}).click();
  await page.getByRole("button",{name:"Finish and log"}).click();
  await expect.poll(async()=> (await notesSnapshot(page)).study.history.length).toBe(1);
  await page.getByRole("button",{name:"Timer",exact:true}).click();
  await page.getByRole("button",{name:"Previous page",exact:true}).click();
  await page.getByLabel("Tool",{exact:true}).selectOption("Rectangle selection");
  await stroke(page,[[.10,.20],[.40,.32]]);
  await expect(page.getByText("1 selected",{exact:true})).toBeVisible();
  await page.getByLabel("Tool",{exact:true}).selectOption("Text");
  await page.getByRole("textbox",{name:"Text to place"}).fill("Study question");
  await stroke(page,[[.19,.43]]);
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].text[0]?.value).toBe("Study question");
  await page.getByText("Edit text boxes",{exact:true}).click();
  await page.getByLabel("Text box 1",{exact:true}).fill("An edited study question");
  await page.getByLabel("Text box 1",{exact:true}).blur();
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].text[0]?.value).toBe("An edited study question");
  await page.getByLabel("Tool",{exact:true}).selectOption("Ink");
  const [backup] = await Promise.all([page.waitForEvent("download"), page.getByRole("button",{name:"Download notebook backup"}).click()]);
  const {readFile}=await import("node:fs/promises");
  const portable=JSON.parse(await readFile(await backup.path(),"utf8"));
  expect(portable.version).toBe(1);
  expect(portable.notebook.pages[0].text[0].value).toBe("An edited study question");
  await page.screenshot({path:info.outputPath("notes-editor-desktop.png")});
  await page.getByRole("button",{name:"← Library",exact:true}).click();
  await expect(notesLibrary(page)).toBeVisible();
  await page.reload();
  await openLocalNotes(page);
  await page.getByRole("button",{name:"Open",exact:true}).click();
  await expect(page.getByLabel("Current page")).toHaveText("1 / 2");
  await expect.poll(async()=> (await notesSnapshot(page)).books[0].pages[0].ink.length).toBe(2);
});

test("cream shelf reflows and selects real notebook fixtures on phone, tablet and desktop",async({page},info)=>{
  test.setTimeout(60000);
  await stubNotesBridge(page,true);
  await page.goto("/");
  await openLocalNotes(page);
  for (const [width,height] of [[320,568],[360,740],[375,812],[390,844],[430,932],[768,1024],[820,1180],[1024,768],[1280,800],[1440,900],[1536,1024],[1920,1080],[844,390]]) {
    await page.setViewportSize({width,height});
    await expect(notesLibrary(page).getByRole("heading",{name:"My Notebooks"})).toBeVisible();
    const results=await page.evaluate(()=>({width:document.documentElement.scrollWidth,viewport:innerWidth}));
    expect(results.width).toBeLessThanOrEqual(results.viewport);
    if ([390,820,1536,844].includes(width)) {
      await page.getByLabel("Notebooks",{exact:true}).focus();
      await page.keyboard.press("ArrowRight");
      await page.keyboard.press("ArrowRight");
      await page.locator(".notes-shelf").blur();
      await expect(page.locator(".notes-book-active")).toHaveCount(1);
      await expect.poll(async()=> (await page.locator(".notes-book-active").boundingBox()).width).toBeGreaterThan(160);
      await page.screenshot({path:info.outputPath(`notes-shelf-${width}.png`),fullPage:true});
    }
  }
  await page.getByRole("searchbox",{name:"Search notebooks"}).fill("Fieldwork");
  await expect(page.locator(".notes-book")).toHaveCount(1);
  await page.getByLabel("Library layout").selectOption("list");
  await expect(page.locator(".notes-list .notes-book")).toHaveCount(1);
});

test("full-screen Notes editor keeps page and focus controls usable across device viewports",async({page},info)=>{
  await stubNotesBridge(page,true);
  await page.goto("/");
  await openLocalNotes(page);
  await page.getByRole("button",{name:"Open",exact:true}).click();
  await expect(notesLibrary(page)).toHaveCount(0);
  for (const [width,height] of [[320,568],[360,740],[390,844],[430,932],[820,1180],[1024,768],[844,390],[1536,1024]]) {
    await page.setViewportSize({width,height});
    expect(await page.evaluate(()=>document.documentElement.scrollWidth)).toBe(width);
    const controls=await page.evaluate(()=>[...document.querySelectorAll(".notes-editor-bar button, .notes-page-bar button, .notes-tray select")]
      .map(item=>{const {left,top,right,bottom,height}=item.getBoundingClientRect();return {left,top,right,bottom,height};}));
    for(const control of controls) {
      expect(control.left).toBeGreaterThanOrEqual(0);
      expect(control.right).toBeLessThanOrEqual(width);
      expect(control.bottom).toBeLessThanOrEqual(height);
      expect(control.height).toBeGreaterThanOrEqual(44);
    }
    const workarea=await page.locator(".notes-workarea").boundingBox();
    expect(workarea.height).toBeGreaterThan(90);
    const pages=page.getByRole("button",{name:"Pages",exact:true});
    await pages.click();
    await expect(page.getByRole("complementary",{name:"Page navigator"})).toBeVisible();
    await pages.click();
    await expect(page.getByRole("complementary",{name:"Page navigator"})).toHaveCount(0);
    if ([360,390,820,844,1024,1536].includes(width)) {
      await page.screenshot({path:info.outputPath(`notes-editor-${width}x${height}.png`)});
    }
  }
  await page.getByRole("button",{name:"Focus",exact:true}).click();
  await expect(page.getByLabel("Writing tools")).toHaveCount(0);
  await expect(page.getByLabel("Writing surface")).toBeVisible();
  await page.getByRole("button",{name:"Show tools",exact:true}).click();
  await expect(page.getByLabel("Writing tools")).toBeVisible();
  await page.getByRole("button",{name:"← Library",exact:true}).click();
  await expect(notesLibrary(page)).toBeVisible();
});

test("recovered Notes remain locked until the checkpoint write is acknowledged",async({page})=>{
  await stubNotesBridge(page,true,true);
  await page.goto("/");
  await page.getByRole("button",{name:"Open Notes on this device"}).click();
  await expect(page.getByRole("heading",{name:"Recover Notes"})).toBeVisible();
  await expect(notesLibrary(page)).toHaveCount(0);
  const recover=page.getByRole("button",{name:"Use recovered checkpoint"});
  await page.evaluate(()=>{window.__failNotesSave=true;});
  await recover.click();
  await expect(page.getByRole("alert")).toHaveText("Fixture storage unavailable.");
  await expect(notesLibrary(page)).toHaveCount(0);

  await page.evaluate(()=>{window.__failNotesSave=false;});
  await recover.click();
  await page.waitForFunction(()=>typeof window.__finishNotesSave==="function");
  await expect(page.getByRole("button",{name:"Saving recovered checkpoint…"})).toBeDisabled();
  await expect(notesLibrary(page)).toHaveCount(0);
  await page.evaluate(()=>window.__finishNotesSave());
  await expect(notesLibrary(page)).toBeVisible();
  await expect(page.getByText("Saved on this device")).toBeVisible();
  await page.reload();
  await openLocalNotes(page);
  await expect(page.getByRole("button",{name:"Archive journal",exact:true})).toBeVisible();
});

test("Notes local folders, cover image, wallpaper, trash and restore survive reload",async({page})=>{
  await stubNotesBridge(page);
  await page.goto("/");
  await openLocalNotes(page);
  const saved=page.getByText("Saved on this device",{exact:true});
  await page.evaluate(()=>{window.__holdNextNotesSave=true;});
  await page.getByRole("button",{name:"Folders",exact:true}).click();
  await page.getByLabel("Folder name",{exact:true}).fill("Research");
  await page.getByRole("button",{name:"Add folder to selected shelf"}).click();
  await page.waitForFunction(()=>typeof window.__finishNotesSave==="function");
  await page.getByLabel("Folder name",{exact:true}).fill("Field work");
  await page.getByRole("button",{name:"Add folder to selected shelf"}).click();
  await expect(page.getByRole("option",{name:"Research / Field work",exact:true})).toBeAttached();
  await expect(page.getByText("Saving…",{exact:true})).toBeVisible();
  expect(await page.evaluate(()=>sessionStorage.getItem("qa-notes"))).toBeNull();
  await page.evaluate(()=>window.__finishNotesSave());
  await expect(saved).toBeVisible();
  const folders=(await notesSnapshot(page)).folders;
  expect(folders).toMatchObject([{title:"Research",parent:null},{title:"Field work",parent:folders[0].id}]);
  await page.getByRole("button",{name:"Close library panel"}).click();
  await page.getByRole("button",{name:"New notebook",exact:true}).click();
  await page.getByLabel("Name",{exact:true}).fill("Research diary");
  await page.getByText("Cover artwork",{exact:true}).click();
  const photo=await picture(page,"cover.png",["#60456b","#45577a","#c0a278"]);
  await page.getByLabel("Choose cover image").setInputFiles(photo);
  await expect(page.locator(".notes-image-preview img")).toHaveAttribute("src",/^blob:/);
  await page.getByRole("button",{name:"Create notebook"}).click();
  await expect(page.locator(".notes-cover-photo")).toHaveAttribute("src",/^blob:/);
  await expect(saved).toBeVisible();
  const [notebook]=(await notesSnapshot(page)).books;
  expect(notebook).toMatchObject({title:"Research diary",folder:folders[1].id,cover_art:expect.any(Object)});

  await page.getByRole("button",{name:"Appearance",exact:true}).click();
  await page.getByLabel("Choose wallpaper").setInputFiles(photo);
  await expect(page.locator(".notes-image-preview img")).toHaveAttribute("src",/^blob:/);
  await page.getByRole("button",{name:"Apply appearance"}).click();
  await expect(page.locator(".notes-wallpaper img")).toHaveAttribute("src",/^blob:/);
  await expect(saved).toBeVisible();
  const appearance=(await notesSnapshot(page)).appearance;
  expect(appearance.wallpaper).toEqual(expect.any(Object));
  await page.reload();
  await openLocalNotes(page);
  await expect(page.locator(".notes-cover-photo")).toHaveAttribute("src",/^blob:/);
  await expect(page.locator(".notes-wallpaper img")).toHaveAttribute("src",/^blob:/);
  await page.getByRole("button",{name:"Folders",exact:true}).click();
  await page.getByRole("combobox",{name:"Shelf",exact:true}).selectOption({label:"Research / Field work"});
  await expect(page.getByRole("button",{name:"Research diary",exact:true})).toBeVisible();
  await page.getByRole("button",{name:"Close library panel"}).click();

  await page.getByRole("button",{name:"Details",exact:true}).click();
  await page.getByLabel("Name",{exact:true}).fill("Research record");
  await page.getByRole("button",{name:"Apply details"}).click();
  await page.getByRole("button",{name:"Details",exact:true}).click();
  await page.getByRole("button",{name:"Move to Recently deleted"}).click();
  await expect(page.getByRole("button",{name:"Research record",exact:true})).toHaveCount(0);
  await expect(saved).toBeVisible();
  expect((await notesSnapshot(page)).books[0]).toMatchObject({id:notebook.id,title:"Research record",disposition:"Trash"});
  await page.reload();
  await openLocalNotes(page);
  await expect(page.getByRole("button",{name:"Research record",exact:true})).toHaveCount(0);
  await page.getByRole("button",{name:"Folders",exact:true}).click();
  await page.getByRole("button",{name:"Recently deleted",exact:true}).click();
  await page.getByRole("button",{name:"Details",exact:true}).click();
  await page.getByRole("button",{name:"Restore to library",exact:true}).click();
  await page.getByRole("button",{name:"Library",exact:true}).click();
  await expect(page.getByRole("button",{name:"Research record",exact:true})).toBeVisible();
  await expect(saved).toBeVisible();
  expect(await notesSnapshot(page)).toMatchObject({
    folders,appearance,books:[{...notebook,title:"Research record",disposition:"Active",modified:expect.any(Number)}],
  });
  await page.reload();
  await openLocalNotes(page);
  await expect(page.getByRole("button",{name:"Research record",exact:true})).toBeVisible();
  await expect(page.locator(".notes-cover-photo")).toHaveAttribute("src",/^blob:/);
  await expect(page.locator(".notes-wallpaper img")).toHaveAttribute("src",/^blob:/);
});

test("Notes exports editable annotation layers into a rotated PDF copy",async({page},info)=>{
  test.setTimeout(60000);
  await stubNotesBridge(page);
  await page.goto("/");
  await openLocalNotes(page);
  const bytes=await notesPdf();
  await page.getByLabel("Import PDF",{exact:true}).setInputFiles({name:"Rotated original.pdf",mimeType:"application/pdf",buffer:bytes});
  await expect(notesEditor(page)).toBeVisible();
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.getByRole("searchbox",{name:"Find in document"}).fill("Second page evidence");
  await page.getByRole("button",{name:"Find",exact:true}).click();
  await page.getByRole("button",{name:"Highlight matching run"}).click();
  await expect(page.getByLabel("Current page")).toHaveText("2 / 2");
  await expect.poll(async()=>(await notesSnapshot(page)).books[0].pages[1].ink.length).toBeGreaterThan(0);
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await stroke(page,[[.45,.22],[.72,.41],[.72,.49]]);
  await expect.poll(async()=>(await notesSnapshot(page)).books[0].pages[1].ink.length).toBeGreaterThan(1);
  const [download]=await Promise.all([page.waitForEvent("download"),page.getByRole("button",{name:"Export PDF",exact:true}).click()]);
  const {readFile}=await import("node:fs/promises");
  const {PDFDocument}=await import("../../web/pdf-runtime/pdf-lib.mjs");
  const exported=await readFile(await download.path());
  const doc=await PDFDocument.load(exported);
  expect(doc.getPageCount()).toBe(2);
  expect(doc.getPage(1).getRotation().angle).toBe(90);
  expect(exported.equals(bytes)).toBe(false);
  await download.saveAs(info.outputPath("annotated-rotated-fixture.pdf"));
  const original=await page.evaluate(()=>Uint8Array.from(atob(sessionStorage.getItem("qa-pdf")),b=>b.charCodeAt(0)));
  expect(Buffer.from(Object.values(original))).toEqual(bytes);
});

test("native Notes vault controls confirm restore and preserve the library on failure",async({page})=>{
  await stubNotesBridge(page,true);
  await page.goto("/");
  await openLocalNotes(page);
  await page.getByRole("button",{name:"Folders",exact:true}).click();
  await page.getByText("Vault backup",{exact:true}).click();
  await page.getByRole("button",{name:"Save vault backup",exact:true}).click();
  await expect(page.getByText(/Vault backup saved to Documents/)).toBeVisible();

  const chooser=page.getByLabel("Restore vault backup",{exact:true});
  const file={name:"fixture.ellipsevault",mimeType:"application/octet-stream",buffer:Buffer.from("corrupt")};
  page.once("dialog",dialog=>dialog.dismiss());
  await chooser.setInputFiles(file);
  await expect(page.getByRole("button",{name:"Archive journal",exact:true})).toBeVisible();
  page.once("dialog",dialog=>dialog.accept());
  await chooser.setInputFiles(file);
  await expect(page.getByText("Invalid fixture backup.")).toBeVisible();
  await expect(page.getByRole("button",{name:"Archive journal",exact:true})).toBeVisible();

  const vault={schema:1,next:1,books:[],folders:[],study:{active:null,history:[]},dark:false};
  page.once("dialog",dialog=>dialog.accept());
  await chooser.setInputFiles({...file,buffer:Buffer.from(JSON.stringify(vault))});
  await expect(page.getByText("Notes vault restored.",{exact:true})).toBeVisible();
  await expect(page.getByRole("heading",{name:"Your shelf is empty",exact:true})).toBeVisible();
  await page.reload();
  await openLocalNotes(page);
  await expect(page.getByRole("heading",{name:"Your shelf is empty",exact:true})).toBeVisible();
});

test("notebook PDF export preserves page stationery and individual pattern colors",async({page},info)=>{
  await stubNotesBridge(page,true);
  await page.goto("/");
  await openLocalNotes(page);
  await page.getByRole("button",{name:"Open",exact:true}).click();
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.getByText("Paper",{exact:true}).click();
  await page.getByRole("combobox",{name:"Template",exact:true}).selectOption("Cornell");
  await page.getByLabel("Pattern color",{exact:true}).fill("#bf1414");
  await page.getByRole("button",{name:"Duplicate page",exact:true}).click();
  await expect(page.getByLabel("Current page")).toHaveText("2 / 2");
  await page.getByLabel("Pattern color",{exact:true}).fill("#1433bf");
  await expect.poll(async()=> (await notesSnapshot(page)).books.find(book=>book.pages.length===2)?.pages
    .map(page=>[page.paper.template,page.paper.pattern])).toEqual([["Cornell",[191,20,20]],["Cornell",[20,51,191]]]);
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  const [download]=await Promise.all([page.waitForEvent("download"),page.getByRole("button",{name:"Export PDF",exact:true}).click()]);
  await download.saveAs(info.outputPath("notebook-stationery-fixture.pdf"));
});

test("300-page image-only PDF stays readable and reports the absence of text",async({page})=>{
  test.setTimeout(60000);
  await stubFirebase(page);
  await page.goto("/");
  await signIn(page);
  await page.getByRole("button",{name:"Notes",exact:true}).click();
  const {PDFDocument}=await import("../../web/pdf-runtime/pdf-lib.mjs");
  const pdf=await PDFDocument.create();
  const scan=await picture(page,"scan.png",["#777","#ccc","#bbb","#eee"]);
  const image=await pdf.embedPng(scan.buffer);
  for(let i=0;i<300;i++){
    const sheet=pdf.addPage([420,530]);
    sheet.drawImage(image,{x:20,y:100,width:380,height:190});
  }
  const start=performance.now();
  await page.getByLabel("Choose PDF",{exact:true}).setInputFiles({
    name:"Scanned fixture.pdf",mimeType:"application/pdf",buffer:Buffer.from(await pdf.save()),
  });
  await expect(page.getByLabel("Current page")).toHaveText("1 / 300");
  await expect.poll(()=>page.locator(".notes-pdf").evaluate(canvas=>canvas.width)).toBeGreaterThan(300);
  console.log(`QA observation: 300-page fixture loaded and rendered in ${Math.round(performance.now()-start)} ms`);
  await page.getByRole("button",{name:"Pages",exact:true}).click();
  await page.getByRole("searchbox",{name:"Find in document"}).fill("missing text");
  await page.getByRole("button",{name:"Find",exact:true}).click();
  await expect(page.getByText("This PDF has no searchable text layer.")).toBeVisible({timeout:15000});
  await page.getByRole("button",{name:"300",exact:true}).click();
  await expect(page.getByLabel("Current page")).toHaveText("300 / 300");
});
