const sdkRoot = "https://www.gstatic.com/firebasejs/13.3.0";
const config = {
  apiKey: "AIzaSyDVZH6QV5vs6pa58NdvV_WO_MWCaMu4jgk",
  authDomain: "ellipse-84ac1.firebaseapp.com",
  projectId: "ellipse-84ac1",
  storageBucket: "ellipse-84ac1.firebasestorage.app",
  messagingSenderId: "90495318702",
  appId: "1:90495318702:web:a63091b13794c9759d4b3f"
};

const subscribers = new Set();
const native = Boolean(window.__TAURI_INTERNALS__);
let snapshot = {type: native ? "signed_out" : "checking"};
let firebase;
let loading;
let unsubscribe;

function publish(update) {
  if (["checking", "signed_out", "signed_in", "unavailable"].includes(update.type)) {
    snapshot = update;
  }
  for (const subscriber of subscribers) subscriber(update);
}

function explain(error) {
  const messages = {
    "auth/unauthorized-domain": "This site must be added to Firebase authorized domains.",
    "auth/operation-not-allowed": "Google sign-in is disabled in Firebase.",
    "auth/popup-blocked": "Your browser blocked the Google sign-in window. Allow pop-ups and retry.",
    "auth/popup-closed-by-user": "The Google window was closed.",
    "auth/cancelled-popup-request": "The previous Google window was closed.",
    "auth/network-request-failed": "Unable to reach Google. Check your internet connection.",
    "auth/too-many-requests": "Too many requests. Try again later.",
    "auth/user-mismatch": "Choose the Google account you used to sign in to Ellipse."
  };
  return messages[error?.code] || "Google could not complete the request. Try again.";
}

function start() {
  if (native || firebase || loading) return;
  publish({type: "checking"});
  const timer = setTimeout(() => {
    publish({type: "unavailable", message: "Google sign-in is taking longer than expected."});
  }, 12000);
  loading = Promise.all([
    import(`${sdkRoot}/firebase-app.js`),
    import(`${sdkRoot}/firebase-auth.js`)
  ]).then(([app, sdk]) => {
    if (!subscribers.size) return;
    const auth = sdk.getAuth(app.getApps().length ? app.getApp() : app.initializeApp(config));
    firebase = {auth, sdk};
    unsubscribe = sdk.onAuthStateChanged(auth, user => {
      clearTimeout(timer);
      publish(user ? {
        type: "signed_in", uid: user.uid, name: user.displayName || "",
        email: user.email || ""
      } : {type: "signed_out"});
    }, error => {
      clearTimeout(timer);
      publish({type: "unavailable", message: explain(error)});
    });
  }).catch(() => {
    clearTimeout(timer);
    publish({type: "unavailable", message: "Could not load Google sign-in. Check your connection and reload Ellipse."});
  }).finally(() => {
    if (!subscribers.size) clearTimeout(timer);
    loading = undefined;
  });
}

export function subscribeAuth(callback) {
  subscribers.add(callback);
  callback(snapshot);
  start();
  return () => {
    subscribers.delete(callback);
    if (subscribers.size) return;
    unsubscribe?.();
    unsubscribe = undefined;
    firebase = undefined;
    snapshot = {type: native ? "signed_out" : "checking"};
  };
}

export function ellipseLogin() {
  if (native) {
    publish({type: "error", message: "Native sign-in is not yet configured. Use Ellipse in your browser to sign in."});
    return;
  }
  if (!firebase) {
    start();
    return;
  }
  const {auth, sdk} = firebase;
  publish({type: "busy"});
  sdk.signInWithPopup(auth, new sdk.GoogleAuthProvider())
    .catch(error => publish({type: "error", message: explain(error)}));
}

export function ellipseLogout() {
  if (!firebase) return;
  const {auth, sdk} = firebase;
  sdk.signOut(auth).catch(error => publish({type: "error", message: explain(error)}));
}

export function isNative() {
  return native;
}

const driveRoot = "https://www.googleapis.com/drive/v3";
const uploadRoot = "https://www.googleapis.com/upload/drive/v3";
const driveScope = "https://www.googleapis.com/auth/drive.appdata";
const backupName = "ellipse-presets-v1.json";
const maxBackupBytes = 3 * 1024 * 1024;

export async function drivePresets(uid, payload) {
  if (native || !firebase || firebase.auth.currentUser?.uid !== uid) {
    throw new Error("Sign in to Ellipse in your browser to access Google Drive.");
  }
  const {auth, sdk} = firebase;
  const provider = new sdk.GoogleAuthProvider();
  provider.addScope(driveScope);
  let result;
  try {
    result = await sdk.reauthenticateWithPopup(auth.currentUser, provider);
  } catch (error) {
    throw new Error(explain(error));
  }
  const token = sdk.GoogleAuthProvider.credentialFromResult(result)?.accessToken;
  if (!token) throw new Error("Google did not grant Drive access.");
  const current = () => {
    if (auth.currentUser?.uid !== uid) throw new Error("The Ellipse account changed.");
  };
  async function request(url, options = {}) {
    current();
    const response = await fetch(url, {
      ...options,
      cache: "no-store",
      credentials: "omit",
      headers: {...options.headers, Authorization: `Bearer ${token}`},
      signal: AbortSignal.timeout(20000)
    });
    current();
    if (response.status === 403) throw new Error("Drive access was denied. Check consent and whether the Drive API is enabled.");
    if (!response.ok) throw new Error(`Google Drive returned HTTP ${response.status}.`);
    return response;
  }
  const query = new URLSearchParams({
    spaces: "appDataFolder",
    q: `name = '${backupName}' and trashed = false`,
    orderBy: "modifiedTime desc",
    fields: "files(id,size)",
    pageSize: "1"
  });
  const {files} = await (await request(`${driveRoot}/files?${query}`)).json();
  const file = files[0];
  if (payload === undefined) {
    if (!file) throw new Error("No Ellipse presets backup was found in Google Drive.");
    if (Number(file.size) > maxBackupBytes) throw new Error("The Drive backup exceeds the preset size limit.");
    const response = await request(`${driveRoot}/files/${encodeURIComponent(file.id)}?alt=media`);
    const blob = await response.blob();
    if (blob.size > maxBackupBytes) throw new Error("The Drive backup exceeds the preset size limit.");
    return blob.text();
  }
  if (new Blob([payload]).size > maxBackupBytes) throw new Error("The presets exceed the Drive backup size limit.");
  if (file) {
    await request(`${uploadRoot}/files/${encodeURIComponent(file.id)}?uploadType=media`, {
      method: "PATCH", headers: {"Content-Type": "application/json"}, body: payload
    });
  } else {
    const boundary = `ellipse-${crypto.randomUUID()}`;
    const metadata = JSON.stringify({name: backupName, parents: ["appDataFolder"], mimeType: "application/json"});
    await request(`${uploadRoot}/files?uploadType=multipart`, {
      method: "POST",
      headers: {"Content-Type": `multipart/related; boundary=${boundary}`},
      body: `--${boundary}\r\nContent-Type: application/json; charset=UTF-8\r\n\r\n${metadata}\r\n--${boundary}\r\nContent-Type: application/json\r\n\r\n${payload}\r\n--${boundary}--\r\n`
    });
  }
}
