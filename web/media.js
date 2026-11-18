// Platform I/O for the Rust artwork editor. Original files never leave the device.
const MAX_BYTES = 24 * 1024 * 1024;
const MAX_PIXELS = 50_000_000;
let database;

function db() {
  if (!database) {
    database = new Promise((resolve, reject) => {
      const request = indexedDB.open("ellipse-local-artwork", 1);
      request.onupgradeneeded = () => request.result.createObjectStore("images");
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(new Error("Close other Ellipse tabs and retry image storage."));
      request.onsuccess = () => {
        const connection = request.result;
        connection.onversionchange = () => { connection.close(); database = undefined; };
        resolve(connection);
      };
    }).catch(error => { database = undefined; throw error; });
  }
  return database;
}

async function transaction(mode, operation) {
  const connection = await db();
  return new Promise((resolve, reject) => {
    const tx = connection.transaction("images", mode);
    const request = operation(tx.objectStore("images"));
    tx.oncomplete = () => resolve(request.result);
    tx.onabort = () => reject(tx.error || request.error || new Error("Image storage was interrupted."));
  });
}

async function rasterType(blob) {
  const b = new Uint8Array(await blob.slice(0, 32).arrayBuffer());
  const four = i => String.fromCharCode(...b.slice(i, i + 4));
  if (b[0] === 255 && b[1] === 216 && b[2] === 255) return "image/jpeg";
  if (b[0] === 137 && four(1) === "PNG\r") return "image/png";
  if (four(0) === "RIFF" && four(8) === "WEBP") return "image/webp";
  if (four(4) === "ftyp" && ["heic", "heix", "hevc", "hevx", "mif1", "msf1"].includes(four(8))) return "image/heic";
  throw new Error("Choose a JPEG, PNG, WebP or compatible HEIC/HEIF image.");
}

const hex = bytes => Array.from(bytes, byte => byte.toString(16).padStart(2, "0")).join("");
async function digest(data) {
  if (crypto.subtle) return hex(new Uint8Array(await crypto.subtle.digest("SHA-256", data)));
  const native = window.__TAURI_INTERNALS__;
  if (native) {
    return native.invoke("hash_artwork", {encoded: await encodeBlob(new Blob([data]))});
  }
  throw new Error("Local artwork needs a secure Ellipse connection.");
}

function encodeBlob(blob) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onload = () => resolve(reader.result.slice(reader.result.indexOf(",") + 1));
    reader.onerror = () => reject(reader.error);
    reader.readAsDataURL(blob);
  });
}

async function loadImage(url) {
  const image = new Image();
  image.src = url;
  await image.decode();
  return image;
}

function sample(image) {
  const canvas = document.createElement("canvas");
  const factor = 48 / Math.max(image.naturalWidth, image.naturalHeight);
  canvas.width = Math.max(1, Math.round(image.naturalWidth * factor));
  canvas.height = Math.max(1, Math.round(image.naturalHeight * factor));
  const context = canvas.getContext("2d", {willReadFrequently: true});
  context.drawImage(image, 0, 0, canvas.width, canvas.height);
  return context.getImageData(0, 0, canvas.width, canvas.height).data;
}

export async function stageImage(file) {
  if (!file || !file.size || file.size > MAX_BYTES) {
    throw new Error("Choose a nonempty image no larger than 24 MiB.");
  }
  const type = await rasterType(file);
  const blob = file.slice(0, file.size, type);
  let url = URL.createObjectURL(blob);
  try {
    let image;
    try {
      image = await loadImage(url);
    } catch {
      throw new Error(type === "image/heic"
        ? "This device cannot display HEIC/HEIF in Ellipse. Export the image as JPEG, PNG or WebP."
        : "This image could not be decoded. Try a different file.");
    }
    if (image.naturalWidth * image.naturalHeight > MAX_PIXELS) {
      throw new Error("Choose an image with fewer than 50 megapixels.");
    }
    const id = await digest(await blob.arrayBuffer());
    // The browser keeps the user's original. Large raster files get a display copy.
    let preview;
    const side = Math.max(image.naturalWidth, image.naturalHeight);
    if (!window.__TAURI_INTERNALS__ && side > 2560 && type !== "image/webp") {
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.round(image.naturalWidth * 2560 / side));
      canvas.height = Math.max(1, Math.round(image.naturalHeight * 2560 / side));
      canvas.getContext("2d").drawImage(image, 0, 0, canvas.width, canvas.height);
      preview = await new Promise(resolve => canvas.toBlob(resolve, "image/webp", 0.94));
      if (preview) {
        URL.revokeObjectURL(url);
        url = URL.createObjectURL(preview);
      }
    }
    return {id, url, blob, preview};
  } catch (error) {
    URL.revokeObjectURL(url);
    throw error;
  }
}

export function releaseStage(stage) {
  URL.revokeObjectURL(stage.url);
}

export function createLibrary(uid) {
  return {uid, entries: new Map()};
}

function accountKey(library) {
  library.account ??= digest(new TextEncoder().encode(library.uid));
  return library.account;
}

async function binaryKey(library, id) {
  if (!/^[0-9a-f]{64}$/.test(id)) throw new Error("This artwork reference is invalid.");
  return [await accountKey(library), id];
}

async function readBlob(library, id) {
  const [account, key] = await binaryKey(library, id);
  const native = window.__TAURI_INTERNALS__;
  let blob;
  if (native) {
    const bytes = await native.invoke("read_artwork", {account, key});
    blob = new Blob([Array.isArray(bytes) ? Uint8Array.from(bytes) : bytes]);
  } else {
    const record = await transaction("readonly", store => store.get(`${account}:${key}`));
    blob = record?.preview || record?.original;
  }
  if (!(blob instanceof Blob) || !blob.size || blob.size > MAX_BYTES) {
    throw new Error("This image is unavailable on this device.");
  }
  return blob.slice(0, blob.size, await rasterType(blob));
}

export async function saveImage(library, stage) {
  const [account, key] = await binaryKey(library, stage.id);
  const native = window.__TAURI_INTERNALS__;
  if (native) {
    // Android WebView's Tauri bridge does not accept raw request bodies.
    await native.invoke("store_artwork", {account, key, encoded: await encodeBlob(stage.blob)});
  } else {
    await transaction("readwrite", store => store.put({original: stage.blob, preview: stage.preview}, `${account}:${key}`));
  }
  return key;
}

export function leaseImage(library, id) {
  let entry = library.entries.get(id);
  if (!entry) {
    entry = {users: 0};
    entry.ready = readBlob(library, id).then(blob => {
      if (entry.users === 0) return null;
      entry.url = URL.createObjectURL(blob);
      return entry.url;
    });
    library.entries.set(id, entry);
  }
  entry.users += 1;
  return {entry, library, id};
}

export function imageReady(lease) { return lease.entry.ready; }

export function releaseImage({entry, library, id}) {
  entry.users -= 1;
  if (entry.users === 0) {
    if (entry.url) URL.revokeObjectURL(entry.url);
    library.entries.delete(id);
  }
}

export async function sampleImage(url) {
  // Give the editor a frame to paint before sampling a bounded canvas.
  await new Promise(resolve => requestAnimationFrame(() => setTimeout(resolve, 0)));
  return sample(await loadImage(url));
}

export function copyHex(hexValue) {
  return navigator.clipboard?.writeText(hexValue) ?? Promise.reject(new Error("Clipboard is unavailable."));
}
