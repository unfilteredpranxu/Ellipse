import {firebaseModules} from "../auth.js";

const button = document.querySelector("#connect");
const status = document.querySelector("#status");
const response = document.querySelector("#response");

function encode(bytes) {
  return btoa(String.fromCharCode(...new Uint8Array(bytes)))
    .replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

async function prepare() {
  const request = location.hash.slice(1);
  history.replaceState(null, "", location.pathname);
  if (!/^v1\.[\w-]{87}$/.test(request)) throw new Error("Start Google sign-in in Ellipse first.");
  const point = Uint8Array.from(atob(request.slice(3).replaceAll("-", "+").replaceAll("_", "/")), c => c.charCodeAt(0));
  const algorithm = {name: "ECDH", namedCurve: "P-256"};
  const appKey = await crypto.subtle.importKey("raw", point, algorithm, false, []);
  const keys = await crypto.subtle.generateKey(algorithm, false, ["deriveBits"]);
  const shared = await crypto.subtle.deriveBits({name: "ECDH", public: appKey}, keys.privateKey, 256);
  const material = await crypto.subtle.importKey("raw", shared, "HKDF", false, ["deriveKey"]);
  const cipher = await crypto.subtle.deriveKey({
    name: "HKDF", hash: "SHA-256", salt: new Uint8Array(),
    info: new TextEncoder().encode("Ellipse native Google pairing v1")
  }, material, {name: "AES-GCM", length: 256}, false, ["encrypt"]);
  const publicKey = encode(await crypto.subtle.exportKey("raw", keys.publicKey));
  const [app, sdk, config] = await firebaseModules();
  const auth = sdk.initializeAuth(app.initializeApp(config, "ellipse-native-pairing"), {
    persistence: sdk.inMemoryPersistence,
    popupRedirectResolver: sdk.browserPopupRedirectResolver
  });
  button.disabled = false;
  button.addEventListener("click", async () => {
    button.disabled = true;
    status.textContent = "Waiting for Google…";
    try {
      const provider = new sdk.GoogleAuthProvider();
      provider.setCustomParameters({prompt: "select_account"});
      const result = await sdk.signInWithPopup(auth, provider);
      const credential = sdk.GoogleAuthProvider.credentialFromResult(result);
      if (!credential?.idToken && !credential?.accessToken) throw new Error("Google returned no credential.");
      const nonce = crypto.getRandomValues(new Uint8Array(12));
      const payload = new TextEncoder().encode(JSON.stringify({idToken: credential.idToken, accessToken: credential.accessToken}));
      const encrypted = await crypto.subtle.encrypt({name: "AES-GCM", iv: nonce}, cipher, payload);
      response.value = `v1.${publicKey}.${encode(nonce)}.${encode(encrypted)}`;
      document.querySelector("#result").hidden = false;
      status.textContent = "Copy the response and return to Ellipse.";
    } catch {
      status.textContent = "Google sign-in could not finish. Allow pop-ups and try again.";
      button.disabled = false;
    } finally {
      await sdk.signOut(auth).catch(() => {});
    }
  });
}
document.querySelector("#copy").addEventListener("click", async () => {
  try {
    await navigator.clipboard.writeText(response.value);
    status.textContent = "Response copied. Paste it into Ellipse.";
  } catch {
    response.select();
    status.textContent = "Copy the selected response manually.";
  }
});
prepare().catch(error => {
  status.textContent = error.message === "Start Google sign-in in Ellipse first."
    ? error.message : "Could not prepare a secure sign-in. Start again in Ellipse.";
});
