/* Static shell only. Auth credentials and Firebase network responses are never cached. */
const CACHE="ellipse-shell-v2";
const FILES=["/","/favicon.svg","/manifest.webmanifest","/assets/ellipse-art.jpg"];
self.addEventListener("install",e=>e.waitUntil(caches.open(CACHE).then(c=>c.addAll(FILES)).then(()=>self.skipWaiting())));
self.addEventListener("activate",e=>e.waitUntil(caches.keys().then(k=>Promise.all(k.filter(x=>x!==CACHE).map(x=>caches.delete(x)))).then(()=>self.clients.claim())));
self.addEventListener("fetch",e=>{
 const u=new URL(e.request.url);
 if(e.request.method!=="GET"||u.origin!==location.origin)return;
 // Network first avoids serving stale OAuth UI after updates; only same-origin public assets are cached.
 e.respondWith(fetch(e.request).then(r=>{
   if(r.ok&&!u.pathname.startsWith("/__/")&&!u.pathname.includes("auth")){const copy=r.clone();caches.open(CACHE).then(c=>c.put(e.request,copy));}
   return r;
 }).catch(async()=>await caches.match(e.request)||Response.error()));
});
