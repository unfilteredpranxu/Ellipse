import {cp, mkdir, rm, copyFile} from "node:fs/promises";
import {execFileSync} from "node:child_process";
import {fileURLToPath} from "node:url";
import path from "node:path";

const root=fileURLToPath(new URL("../../",import.meta.url));
const web=path.join(root,"web");
const out=path.join(root,"native","dist");
await mkdir(path.join(web,"assets"),{recursive:true});
await cp(path.join(root,"site","assets"),path.join(web,"assets"),{recursive:true,force:true});
await rm(out,{recursive:true,force:true});
execFileSync("trunk",["build","--release","--locked","--dist",out],{cwd:web,stdio:"inherit"});
for(const name of ["favicon.svg","manifest.webmanifest","sw.js","_headers"]){
  await copyFile(path.join(root,"site",name),path.join(out,name));
}
console.log("Ellipse native: compiled Leptos Rust WebAssembly UI ready in native/dist");
