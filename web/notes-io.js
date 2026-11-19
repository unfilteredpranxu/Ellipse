// Browser rendering and Tauri transport for the Rust Notes model.
const limit = 24 * 1024 * 1024;
let engine;
function pdfEngine() {
  return engine ??= import("/pdf-runtime/pdf.min.mjs").then(pdfjs => {
    pdfjs.GlobalWorkerOptions.workerSrc = "/pdf-runtime/pdf.worker.min.mjs";
    return pdfjs;
  });
}

export function invokeNotes(command, args) {
  const api = window.__TAURI_INTERNALS__;
  if (!api || typeof api.invoke !== "function") {
    return Promise.reject(new Error("Private Notes storage needs the installed Ellipse app."));
  }
  return api.invoke(command, args);
}

function checked(bytes) {
  const ascii = new TextDecoder("latin1");
  if (bytes.length > limit || !ascii.decode(bytes.subarray(0, 5)).startsWith("%PDF-")
      || !ascii.decode(bytes.subarray(Math.max(0, bytes.length - 4096))).includes("%%EOF")) {
    throw new Error("Choose a complete PDF up to 24 MiB.");
  }
  return bytes;
}

async function parse(bytes) {
  checked(bytes);
  const pdfjs = await pdfEngine();
  const task = pdfjs.getDocument({
    data: bytes.slice(), isEvalSupported: false, enableXfa: false,
    useSystemFonts: true, maxImageSize: 40_000_000,
    cMapUrl: "/pdf-runtime/cmaps/", cMapPacked: true,
    standardFontDataUrl: "/pdf-runtime/standard_fonts/",
    wasmUrl: "/pdf-runtime/wasm/", iccUrl: "/pdf-runtime/iccs/",
  });
  try {
    const doc = await task.promise;
    if (!doc.numPages || doc.numPages > 1000) {
      throw new Error("Choose a PDF with no more than 1,000 pages.");
    }
    return {doc, bytes};
  } catch (error) {
    await task.destroy();
    if (error?.name === "PasswordException") {
      throw new Error("Password-protected PDFs are unavailable in this version.");
    }
    throw error;
  }
}

export async function readLocalPdf(file) {
  if (!file || file.size > limit) throw new Error("Choose a PDF up to 24 MiB.");
  return parse(new Uint8Array(await file.arrayBuffer()));
}

export async function readStoredPdf(id) {
  const result = await invokeNotes("notes_read_pdf", {id});
  return parse(new Uint8Array(result));
}

export async function pdfInfo(pdf) {
  const pages = [];
  for (let i = 1; i <= pdf.doc.numPages; i++) {
    const page = await pdf.doc.getPage(i);
    const view = page.getViewport({scale:1});
    if (!(view.width >= 12 && view.height >= 12 && view.width <= 5000 && view.height <= 5000)) {
      throw new Error("This PDF has a page size Ellipse cannot display.");
    }
    pages.push({width:view.width,height:view.height,matrix:view.transform});
    page.cleanup();
  }
  return pages;
}

export function closePdf(pdf) {
  pdf.rendering?.cancel();
  pdf.doc.destroy().catch(() => {});
}

export async function renderPdf(pdf, number, canvas) {
  const generation = (pdf.paintGeneration ?? 0) + 1;
  pdf.paintGeneration = generation;
  pdf.rendering?.cancel();
  const page = await pdf.doc.getPage(number);
  if (generation !== pdf.paintGeneration || !canvas.isConnected) return;
  const size = page.getViewport({scale:1});
  const scale = Math.min(Math.max(canvas.clientWidth / size.width * devicePixelRatio, 0.5), Math.sqrt(4_000_000 / (size.width * size.height)));
  const viewport = page.getViewport({scale});
  canvas.width = Math.ceil(viewport.width);
  canvas.height = Math.ceil(viewport.height);
  const task = page.render({canvas, viewport, annotationMode: 0});
  pdf.rendering = task;
  try {
    await task.promise;
  } catch (error) {
    if (error?.name !== "RenderingCancelledException") throw error;
  } finally {
    if (pdf.rendering === task) pdf.rendering = undefined;
  }
}

export function pdfOutline(pdf) {
  // Cache only sanitized destinations within this document.
  return pdf.outline ??= readOutline(pdf.doc).catch(error => {
    pdf.outline = undefined;
    throw error;
  });
}

async function readOutline(doc) {
  const items = await doc.getOutline();
  const result = [];
  let remaining = 500;
  async function visit(entries, depth) {
    if (depth >= 16 || !entries) return;
    for (const entry of entries) {
      if (--remaining < 0) return;
      let page = null;
      if (!entry.url && entry.dest) {
        try {
          const dest = typeof entry.dest === "string" ? await doc.getDestination(entry.dest) : entry.dest;
          if (Array.isArray(dest)) {
            const ref = dest[0];
            const index = Number.isInteger(ref) ? ref : ref && await doc.getPageIndex(ref);
            if (Number.isInteger(index) && index >= 0 && index < doc.numPages) page = index + 1;
          }
        } catch {
          // A broken outline entry must not block the rest of the document.
        }
      }
      const title = String(entry.title ?? "").replace(/[\x00-\x1f\x7f]/g, " ").trim().slice(0, 240);
      if (title && (page !== null || entry.items?.length)) result.push({title, page, depth});
      await visit(entry.items, depth + 1);
    }
  }
  await visit(items, 0);
  return result;
}

const thumbnailJobs = new WeakMap();

export function cancelPdfThumbnail(canvas) {
  const job = thumbnailJobs.get(canvas);
  thumbnailJobs.delete(canvas);
  job?.task?.cancel();
}

export async function renderPdfThumbnail(pdf, number, canvas) {
  cancelPdfThumbnail(canvas);
  const job = {};
  thumbnailJobs.set(canvas, job);
  try {
    const page = await pdf.doc.getPage(number);
    if (thumbnailJobs.get(canvas) !== job || !canvas.isConnected) return;
    const size = page.getViewport({scale:1});
    if (!Number.isFinite(size.width) || !Number.isFinite(size.height) || size.width <= 0 || size.height <= 0) {
      throw new Error("The PDF page dimensions could not be read.");
    }
    const viewport = page.getViewport({scale:Math.min(128 / size.width, 112 / size.height)});
    canvas.width = Math.max(1, Math.min(128, Math.ceil(viewport.width)));
    canvas.height = Math.max(1, Math.min(112, Math.ceil(viewport.height)));
    job.task = page.render({canvas, viewport, annotationMode: 0});
    await job.task.promise;
    if (thumbnailJobs.get(canvas) === job) canvas.dataset.rendered = "true";
  } catch (error) {
    if (thumbnailJobs.get(canvas) === job && error?.name !== "RenderingCancelledException") throw error;
  } finally {
    if (thumbnailJobs.get(canvas) === job) thumbnailJobs.delete(canvas);
  }
}

export async function findPdf(pdf, query) {
  const needle = query.trim().toLocaleLowerCase();
  const hits = [];
  let hasText = false;
  for (let i = 1; i <= pdf.doc.numPages; i++) {
    const page = await pdf.doc.getPage(i);
    const content = await page.getTextContent();
    const text = content.items.map(item => item.str ?? "").join(" ").trim();
    hasText ||= Boolean(text);
    const match = text.toLocaleLowerCase().indexOf(needle);
    if (needle && match >= 0 && hits.length < 100) {
      const view = page.getViewport({scale:1});
      const runs = [];
      for (const item of content.items) {
        if (!item.str?.toLocaleLowerCase().includes(needle) || !item.width || !item.height) continue;
        const [a,b,c,d,x,y] = item.transform;
        const along = Math.hypot(a,b);
        const up = Math.hypot(c,d);
        if (!along || !up) continue;
        const endX = x + a/along*item.width;
        const endY = y + b/along*item.width;
        const center = (px,py,level) => view.convertToViewportPoint(px+c/up*item.height*level,py+d/up*item.height*level);
        runs.push({
          highlight:[center(x,y,.40),center(endX,endY,.40)],
          underline:[center(x,y,-.12),center(endX,endY,-.12)],
          strike:[center(x,y,.32),center(endX,endY,.32)],
          width:Math.min(72,Math.max(1,item.height*.95)),
        });
        if (runs.length>=100) break;
      }
      hits.push({page:i,text:text.slice(Math.max(0,match-30),match+needle.length+52),runs});
    }
    if (i % 8 === 0) await new Promise(resolve => setTimeout(resolve, 0));
  }
  return {hasText,hits};
}

export function storePdf(pdf) {
  return new Promise((resolve,reject) => {
    const file = new FileReader();
    file.onerror = () => reject(new Error("The PDF could not be staged."));
    file.onload = () => resolve(invokeNotes("notes_store_pdf", {encoded:file.result.slice(file.result.indexOf(",")+1)}));
    file.readAsDataURL(new Blob([pdf.bytes]));
  });
}

function download(bytes, name, type) {
  const url = URL.createObjectURL(new Blob([bytes],{type}));
  const link = document.createElement("a");
  link.href=url;
  link.download=name.replace(/[\\/:*?"<>|]/g,"_");
  document.body.append(link);
  link.click();
  link.remove();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}

export function downloadOriginal(pdf, name) {
  download(pdf.bytes, name, "application/pdf");
}

export function downloadNotebook(json, name) {
  download(json, name, "application/json");
}

export async function restoreNotes(head, file) {
  if (!file?.size || file.size > 96 * 1024 * 1024) {
    throw new Error("Choose a Notes vault backup up to 96 MiB.");
  }
  const encoded = await new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(new Error("The Notes backup could not be read."));
    reader.onload = () => resolve(reader.result.slice(reader.result.indexOf(",") + 1));
    reader.readAsDataURL(file);
  });
  return invokeNotes("notes_restore", {head, encoded});
}

export async function exportPdf(original, spec, stationery, name) {
  const {PDFDocument, StandardFonts, rgb, degrees, drawObject,
    setFillingColor, pushGraphicsState, popGraphicsState} = await import("/pdf-runtime/pdf-lib.mjs");
  const output = await PDFDocument.create();
  if (original) {
    const incoming = await PDFDocument.load(original.bytes, {updateMetadata:false});
    const pages = await output.copyPages(incoming,incoming.getPageIndices());
    for (const page of pages) output.addPage(page);
  }
  const fonts = await Promise.all([StandardFonts.Helvetica,StandardFonts.HelveticaBold,
    StandardFonts.HelveticaOblique,StandardFonts.HelveticaBoldOblique].map(font => output.embedFont(font)));
  const color = values => rgb(...values.map(n => n/255));
  const stationeryForms = new Map();
  for (let i=0; i<spec.length; i++) {
    const layer = spec[i];
    const page = original ? output.getPage(i) : output.addPage([layer.width,layer.height]);
    if (!original) {
      page.drawRectangle({x:0,y:0,width:layer.width,height:layer.height,color:color(layer.background)});
      if (!stationeryForms.has(layer.stationery)) {
        // Reuse the compressed vector pattern on pages with matching geometry.
        const marks = stationery(i);
        const data = marks.map(([x,y,w,h]) => [x,layer.height-y-h,w,h]
          .map(value => value.toFixed(4)).join(" ") + " re").join("\n");
        const form = data ? output.context.register(output.context.flateStream(data + "\nf", {
          Type:"XObject", Subtype:"Form", BBox:[0,0,layer.width,layer.height],
        })) : null;
        stationeryForms.set(layer.stationery,form);
      }
      const form = stationeryForms.get(layer.stationery);
      if (form) {
        const key = page.node.newXObject("EllipsePaper",form);
        page.pushOperators(pushGraphicsState(),setFillingColor(color(layer.pattern)),drawObject(key),popGraphicsState());
      }
    }
    for (const mark of layer.ink) {
      if (mark.points.length < 3) continue;
      const path=mark.points.map(([x,y],index) => `${index ? "L" : "M"}${x.toFixed(4)} ${(-y).toFixed(4)}`).join(" ")+" Z";
      page.drawSvgPath(path,{x:0,y:0,color:color(mark.color),opacity:mark.opacity});
    }
    for (const text of layer.text) {
      try {
        page.drawText(text.value,{x:text.x,y:text.y,rotate:degrees(text.angle),size:text.size,font:fonts[Number(text.bold)+Number(text.italic)*2],color:color(text.color)});
      } catch {
        throw new Error("This PDF export cannot encode one of the text boxes. Your editable document is intact.");
      }
    }
  }
  download(await output.save(), name, "application/pdf");
}
