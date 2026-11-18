import { cp, mkdir } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { join } from "node:path";

const root = fileURLToPath(new URL("../web/pdf/", import.meta.url));
const dst = fileURLToPath(new URL("../web/pdf-runtime/", import.meta.url));
execFileSync(process.platform === "win32" ? "npm.cmd" : "npm",
  ["ci", "--ignore-scripts", "--omit=optional", "--no-audit", "--no-fund"],
  {cwd:root, stdio:"inherit", shell:process.platform === "win32"});
const pdfjs = join(root,"node_modules/pdfjs-dist");
const pdflib = join(root,"node_modules/pdf-lib");
await mkdir(dst,{recursive:true});
for (const file of ["pdf.min.mjs","pdf.worker.min.mjs"]) {
  await cp(join(pdfjs,"legacy/build",file),join(dst,file));
}
for (const dir of ["cmaps","standard_fonts","wasm","iccs"]) {
  await cp(join(pdfjs,dir),join(dst,dir),{recursive:true});
}
await cp(join(pdflib,"dist/pdf-lib.esm.min.js"),join(dst,"pdf-lib.mjs"));
await cp(join(pdfjs,"LICENSE"),join(dst,"PDF.js-LICENSE.txt"));
await cp(join(pdflib,"LICENSE.md"),join(dst,"pdf-lib-LICENSE.txt"));
