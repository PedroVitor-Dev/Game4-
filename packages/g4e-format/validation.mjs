import { validateG4epDocument } from "./g4ep-validation.mjs";
// Offline reference validator. Never opens companion torrents, payloads or URLs.
import { readFileSync } from "node:fs";
import Ajv2020 from "ajv/dist/2020.js";
export const MAX_BYTES = 64 * 1024 * 1024;
const MAX_MANIFEST_BYTES = 32 * 1024 * 1024;
const MAX_METAINFO_BYTES = 8n * 1024n * 1024n;
const U64_MAX = 18446744073709551615n;
const MAX_G4_BYTES = 1099511627776n + 512n * 1024n * 1024n;
const manifestSchema = JSON.parse(
  readFileSync(
    new URL("../manifest-schema/v1.schema.json", import.meta.url),
    "utf8",
  ),
);
const schema = JSON.parse(
  readFileSync(new URL("./v1.schema.json", import.meta.url), "utf8"),
);
const ajv = new Ajv2020({ strict: true, allErrors: false });
ajv.addSchema(manifestSchema);
const structural = ajv.compile(schema);
const structuralV2 = ajv.compile(
  JSON.parse(
    readFileSync(new URL("./v2.schema.json", import.meta.url), "utf8"),
  ),
);
export class G4eValidationError extends Error {
  constructor(code, message) {
    super(message);
    this.name = "G4eValidationError";
    this.code = code;
  }
}
function fail(message, code = "G4E_INVALID") {
  throw new G4eValidationError(code, message);
}
// Scan before JSON.parse to enforce depth and duplicate keys (including escaped
// spellings). The built-in parser subsequently validates full JSON grammar.
function scan(text, canonicalIntegers = false) {
  let i = 0;
  let manifestRange;
  function ws() {
    while (i < text.length && /[ \t\r\n]/.test(text[i])) i++;
  }
  function string() {
    const start = i;
    if (text[i++] !== '"') fail("Expected a JSON string");
    while (i < text.length) {
      const c = text[i++];
      if (c === '"') return text.slice(start, i);
      if (c === "\\") i++;
    }
    fail("Unterminated JSON string");
  }
  function value(depth) {
    if (depth > 64) fail("JSON nesting exceeds 64", "G4E_LIMIT_EXCEEDED");
    ws();
    const c = text[i];
    if (c === '"') {
      string();
      return;
    }
    if (c === "{") {
      i++;
      ws();
      if (text[i] === "}") {
        i++;
        return;
      }
      const keys = new Set();
      while (i < text.length) {
        ws();
        const key = JSON.parse(string());
        if (keys.has(key)) fail("Duplicate JSON key");
        keys.add(key);
        ws();
        if (text[i++] !== ":") fail("Expected a colon");
        ws();
        const start = i;
        value(depth + 1);
        if (depth === 0 && key === "manifest") manifestRange = [start, i];
        ws();
        const delimiter = text[i++];
        if (delimiter === "}") return;
        if (delimiter !== ",") fail("Expected an object delimiter");
      }
      fail("Unterminated JSON object");
    }
    if (c === "[") {
      i++;
      ws();
      if (text[i] === "]") {
        i++;
        return;
      }
      while (i < text.length) {
        value(depth + 1);
        ws();
        const delimiter = text[i++];
        if (delimiter === "]") return;
        if (delimiter !== ",") fail("Expected an array delimiter");
      }
      fail("Unterminated JSON array");
    }
    const start = i;
    while (i < text.length && !/[ \t\r\n,}\]]/.test(text[i])) i++;
    if (start === i) fail("Expected a JSON value");
    const token = text.slice(start, i);
    if (
      canonicalIntegers &&
      !["true", "false", "null"].includes(token) &&
      !/^(0|[1-9][0-9]*)$/.test(token)
    )
      fail("v2 numeric fields require unsigned decimal integer tokens");
  }
  value(0);
  ws();
  if (i !== text.length) fail("Unexpected trailing data");
  return manifestRange;
}
function unicode(value) {
  if (typeof value === "string") {
    if (/[\uD800-\uDFFF]/u.test(value)) fail("Unpaired Unicode surrogate");
  } else if (typeof value === "number" && !Number.isFinite(value))
    fail("Non-finite JSON number");
  else if (value && typeof value === "object")
    for (const [k, v] of Object.entries(value)) {
      unicode(k);
      unicode(v);
    }
}
function portable(path) {
  if (Buffer.byteLength(path, "utf8") > 4096 || path.split("/").length > 128)
    fail("Path exceeds profile limits", "G4E_LIMIT_EXCEEDED");
}
function tracker(text) {
  let url;
  try {
    url = new URL(text);
  } catch {
    fail("Invalid tracker URL");
  }
  if (
    !["https:", "http:", "udp:"].includes(url.protocol) ||
    !url.hostname ||
    url.username ||
    url.password ||
    url.hash ||
    /[\u0000-\u0020\u007f]/.test(text) ||
    url.port === "0" ||
    (url.protocol === "udp:" && !url.port)
  )
    fail("Unsupported tracker URL");
}
function magnet(uri, hash) {
  if (
    !/^magnet:\?[^\s\u0080-\uffff]*$/.test(uri) ||
    /%(?![0-9a-fA-F]{2})/.test(uri)
  )
    fail("Invalid magnet encoding");
  let url;
  try {
    url = new URL(uri);
  } catch {
    fail("Invalid magnet URI");
  }
  if (url.protocol !== "magnet:" || url.host || url.pathname || url.hash)
    fail("Invalid magnet URI");
  const p = url.searchParams;
  for (const [k, v] of p) {
    if (
      !["xt", "dn", "tr"].includes(k) ||
      /[\u0000-\u001f\u007f-\u009f\ufffd]/.test(v)
    )
      fail("Unsupported magnet parameter");
  }
  if (
    p.getAll("xt").length !== 1 ||
    p.get("xt") !== `urn:btih:${hash}` ||
    p.getAll("dn").length > 1 ||
    p.getAll("tr").length > 32
  )
    fail("Magnet identity or bounds mismatch");
  const trackers = p.getAll("tr");
  if (new Set(trackers).size !== trackers.length) fail("Duplicate trackers");
  trackers.forEach(tracker);
}
export function parseG4e(input) {
  try {
    if (typeof input !== "string" && !(input instanceof Uint8Array))
      fail("Expected JSON text or UTF-8 bytes");
    if (
      (typeof input === "string"
        ? Buffer.byteLength(input, "utf8")
        : input.byteLength) > MAX_BYTES
    )
      fail("Descriptor exceeds 64 MiB", "G4E_LIMIT_EXCEEDED");
    const text =
      typeof input === "string"
        ? input
        : new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(
            input,
          );
    const range = scan(text);
    if (
      range &&
      Buffer.byteLength(text.slice(...range), "utf8") > MAX_MANIFEST_BYTES
    )
      fail("Embedded manifest exceeds 32 MiB", "G4E_LIMIT_EXCEEDED");
    const doc = JSON.parse(text);
    unicode(doc);
    if (doc?.schemaVersion === 2) {
      scan(text, true);
      if (
        doc.g4ep &&
        Number.isInteger(doc.g4ep.protocolVersion) &&
        doc.g4ep.protocolVersion !== 1
      )
        fail("Unsupported G4EP version", "G4E_UNSUPPORTED_VERSION");
      if (!structuralV2(doc)) fail("Descriptor does not match v2 schema");
      return validateG4epDocument(doc, fail);
    }
    if (doc && Number.isInteger(doc.schemaVersion) && doc.schemaVersion !== 1)
      fail("Unsupported .g4e version", "G4E_UNSUPPORTED_VERSION");
    if (!structural(doc)) fail("Descriptor does not match the public schema");
    const paths = new Set();
    let total = 0n;
    for (const file of doc.manifest.files) {
      portable(file.path);
      const path = file.path.toLowerCase();
      if (paths.has(path)) fail("Duplicate payload path");
      paths.add(path);
      const size = BigInt(file.sizeBytes);
      if (size > U64_MAX) fail("File size exceeds u64");
      total += size;
      if (total > U64_MAX) fail("Total payload size exceeds u64");
    }
    for (const path of paths) {
      let i = path.indexOf("/");
      while (i >= 0) {
        if (paths.has(path.slice(0, i))) fail("File/directory path collision");
        i = path.indexOf("/", i + 1);
      }
    }
    if (
      doc.representation === "g4_package" &&
      (doc.manifest.files.length !== 1 ||
        !/\.g4$/i.test(doc.manifest.files[0].path))
    )
      fail("Packaged representation requires exactly one .g4 payload");
    if (doc.representation === "g4_package" && total > MAX_G4_BYTES)
      fail("Package exceeds .g4 v1 size limit", "G4E_LIMIT_EXCEEDED");
    const hashes = new Set();
    const companions = new Set();
    for (const torrent of doc.torrents) {
      if (hashes.has(torrent.infoHashV1)) fail("Duplicate torrent identity");
      hashes.add(torrent.infoHashV1);
      magnet(torrent.magnet, torrent.infoHashV1);
      if (torrent.metainfoFile !== null) {
        const meta = torrent.metainfoFile;
        portable(meta.path);
        if (
          !/\.torrent$/i.test(meta.path) ||
          paths.has(meta.path.toLowerCase())
        )
          fail("Unsafe metainfo companion path");
        const metaPath = meta.path.toLowerCase();
        if (companions.has(metaPath)) fail("Duplicate metainfo companion path");
        for (const other of companions)
          if (
            other.startsWith(metaPath + "/") ||
            metaPath.startsWith(other + "/")
          )
            fail("Metainfo companion prefix collision");
        companions.add(metaPath);
        for (const path of paths)
          if (
            path.startsWith(metaPath + "/") ||
            metaPath.startsWith(path + "/")
          )
            fail("Metainfo/payload prefix collision");
        if (BigInt(meta.sizeBytes) > MAX_METAINFO_BYTES)
          fail("Metainfo companion exceeds 8 MiB", "G4E_LIMIT_EXCEEDED");
      }
    }
    return doc;
  } catch (error) {
    if (error instanceof G4eValidationError) throw error;
    fail("Invalid JSON or UTF-8 input");
  }
}
