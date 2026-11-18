import { cp, mkdir, rm } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
const site = fileURLToPath(new URL('../../site/', import.meta.url));
const out = fileURLToPath(new URL('../dist/', import.meta.url));
await rm(out, { recursive: true, force: true });
await mkdir(out, { recursive: true });
await cp(site, out, { recursive: true });
console.log('Ellipse: copied shared web interface to native frontendDist');
