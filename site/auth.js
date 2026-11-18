import {initializeApp} from "https://www.gstatic.com/firebasejs/13.3.0/firebase-app.js";
import {getAuth,GoogleAuthProvider,onAuthStateChanged,signInWithPopup,signOut} from "https://www.gstatic.com/firebasejs/13.3.0/firebase-auth.js";

const config={
  apiKey:"AIzaSyDVZH6QV5vs6pa58NdvV_WO_MWCaMu4jgk",
  authDomain:"ellipse-84ac1.firebaseapp.com",
  projectId:"ellipse-84ac1",
  storageBucket:"ellipse-84ac1.firebasestorage.app",
  messagingSenderId:"90495318702",
  appId:"1:90495318702:web:a63091b13794c9759d4b3f"
};
let auth;
function broadcast(detail){
  window.ellipseAuthSnapshot=detail;
  window.dispatchEvent(new CustomEvent("ellipse:auth",{detail}));
}
function explain(e){
  const code=e?.code||"auth/unknown";
  const known={
    "auth/unauthorized-domain":"This site must be added to Firebase authorized domains.",
    "auth/operation-not-allowed":"Google sign-in is disabled in Firebase.",
    "auth/popup-blocked":"Your browser blocked the Google sign-in window. Allow pop-ups and retry.",
    "auth/popup-closed-by-user":"Google sign-in was cancelled.",
    "auth/cancelled-popup-request":"The previous sign-in window was closed.",
    "auth/network-request-failed":"Unable to reach Google. Check your internet connection.",
    "auth/too-many-requests":"Too many requests. Try again later.",
    "auth/invalid-api-key":"The Firebase configuration needs correction."
  };
  return (known[code]||"Google sign-in could not be completed.")+" ("+code+")";
}
window.ellipseLogin=()=>{
  if(window.__TAURI_INTERNALS__){
    broadcast({type:"error",message:"Native sign-in is not yet configured. The browser version supports Google sign-in."});
    return;
  }
  if(!auth){broadcast({type:"error",message:"Authentication is loading. Try again."});return;}
  broadcast({type:"busy"});
  // Start the popup in the original click gesture; don't await an unrelated promise.
  signInWithPopup(auth,new GoogleAuthProvider()).catch(e=>broadcast({type:"error",message:explain(e)}));
};
window.ellipseLogout=()=>{
  if(!auth)return;
  signOut(auth).catch(e=>broadcast({type:"error",message:explain(e)}));
};
try{
  auth=getAuth(initializeApp(config));
  onAuthStateChanged(auth,user=>{
    broadcast(user?{type:"signed_in",name:user.displayName||"Google account",email:user.email||"",uid:user.uid}:{type:"signed_out"});
  },e=>broadcast({type:"error",message:explain(e)}));
}catch(e){broadcast({type:"error",message:explain(e)});}
