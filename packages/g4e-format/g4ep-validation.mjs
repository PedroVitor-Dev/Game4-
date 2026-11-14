// .g4e v2 G4EP semantics. No I/O or URL lookup from descriptor values.
import { createHash, createPublicKey, verify } from "node:crypto";
import { isIP } from "node:net";
export const G4EP_CODEC = "g4ep-rs16-leopard-high-8-4-v1";
export const CANONICALIZATION = "game4-g4e-v2-canonical-json-v1";
export const signatureDomain = Buffer.from("Game4.g4e.v2\0");
const hexHash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const identity = (sha) => `urn:sha256:${sha}`;
export function canonicalJson(value) {
  if (value === null || typeof value !== "object") return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  return `{${Object.keys(value)
    .sort()
    .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
    .join(",")}}`;
}
export function g4epPayloadBytes(doc) {
  const { authentication, ...payload } = doc;
  void authentication;
  return Buffer.from(canonicalJson(payload), "utf8");
}
export function g4epPayloadHash(doc) {
  return hexHash(g4epPayloadBytes(doc));
}
export function signatureMessage(doc) {
  return Buffer.concat([
    signatureDomain,
    Buffer.from(doc.authentication.payloadSha256, "hex"),
  ]);
}
function peer(endpoint, fail) {
  let address;
  try {
    address = new URL(`tcp://${endpoint}`);
  } catch {
    fail("Invalid private endpoint");
  }
  if (
    address.username ||
    address.password ||
    address.pathname ||
    address.search ||
    address.hash ||
    !address.port ||
    address.port === "0" ||
    Number(address.port) > 65535 ||
    `${address.host}` !== endpoint ||
    endpoint.includes("%")
  )
    fail("Noncanonical private endpoint");
  const host = address.hostname.replace(/^\[|\]$/g, "");
  const kind = isIP(host);
  let allowed = false;
  if (kind === 4) {
    const p = host.split(".").map(Number);
    allowed =
      p[0] === 10 ||
      p[0] === 127 ||
      (p[0] === 172 && p[1] >= 16 && p[1] <= 31) ||
      (p[0] === 192 && p[1] === 168) ||
      (p[0] === 169 && p[1] === 254) ||
      (p[0] === 100 && p[1] >= 64 && p[1] <= 127);
  }
  if (kind === 6) {
    const first = Number.parseInt(host.split(":")[0] || "0", 16);
    allowed =
      host === "::1" ||
      (first >= 0xfc00 && first <= 0xfdff) ||
      (first >= 0xfe80 && first <= 0xfebf);
  }
  if (!allowed) fail("Peer must use an explicit private address");
  return endpoint;
}
// Encoding checks supplement OpenSSL verification; no curve/group implementation is supplied here.
const lowOrderY = new Set([
  "00".repeat(32),
  "01" + "00".repeat(31),
  "ec" + "ff".repeat(30) + "7f",
  "26e8958fc2b227b045c3f489f2ef98f0d5dfac05d3c63339b13802886d53fc05",
  "c7176a703d4dd84fba3c0b760d10670f2a2053fa2c39ccc64ec7fd7792ac037a",
]);
function strictPointEncoding(bytes) {
  if (bytes.length !== 32) return false;
  const y = Buffer.from(bytes);
  y[31] &= 0x7f;
  const modulus = Buffer.from("ed" + "ff".repeat(30) + "7f", "hex");
  return (
    Buffer.compare(Buffer.from(y).reverse(), Buffer.from(modulus).reverse()) <
      0 && !lowOrderY.has(y.toString("hex"))
  );
}
export function validateG4epDocument(doc, fail) {
  const original = doc.manifest.files[0],
    coding = doc.g4ep.coding;
  if (
    original.path.includes("/") ||
    Buffer.byteLength(original.path, "utf8") > 255 ||
    doc.contentId !== identity(original.sha256)
  )
    fail("Original identity/base name mismatch");
  const size = BigInt(original.sizeBytes),
    shard = BigInt(coding.shardBytes),
    width = 8n * shard;
  if (size > 1099511627776n) fail("Source exceeds limit");
  if (size === 0n && original.sha256 !== hexHash(Buffer.alloc(0)))
    fail("Invalid empty object hash", "G4E_INTEGRITY_FAILED");
  const groups = (size + width - 1n) / width;
  if (groups * 12n > 1000000n || BigInt(doc.g4ep.groups.length) !== groups)
    fail("Group count mismatch");
  const fragmentIds = new Set();
  for (const [i, group] of doc.g4ep.groups.entries()) {
    const index = BigInt(i),
      offset = index * width,
      length = size - offset < width ? size - offset : width;
    if (
      group.id !== `g${String(i).padStart(12, "0")}` ||
      group.index !== String(i) ||
      group.originalOffset !== offset.toString() ||
      group.originalBytes !== length.toString()
    )
      fail("Noncontiguous or reordered group");
    for (const [j, f] of group.fragments.entries()) {
      if (
        f.index !== j ||
        f.sizeBytes !== String(coding.shardBytes) ||
        f.id !== identity(f.sha256)
      )
        fail("Invalid fragment coordinate/identity");
      fragmentIds.add(f.id);
    }
  }
  const seen = new Set(),
    nodeIds = new Set(),
    endpoints = new Set();
  for (const reference of doc.discovery) {
    const entry = canonicalJson(reference);
    if (seen.has(entry)) fail("Duplicate discovery reference");
    seen.add(entry);
    if (reference.kind === "private_peer") {
      const endpoint = peer(reference.endpoint, fail);
      if (nodeIds.has(reference.nodeId) || endpoints.has(endpoint))
        fail("Duplicate private node/endpoint");
      nodeIds.add(reference.nodeId);
      endpoints.add(endpoint);
      if (nodeIds.size > 32) fail("Private peer limit");
    } else if (
      !fragmentIds.has(reference.fragmentId) ||
      reference.magnet !== `magnet:?xt=urn:btih:${reference.infoHashV1}`
    )
      fail("Unbound fragment torrent reference");
  }
  if (doc.authentication.payloadSha256 !== g4epPayloadHash(doc))
    fail("Metadata digest mismatch", "G4E_INTEGRITY_FAILED");
  let previous = "";
  for (const signature of doc.authentication.signatures) {
    if (previous && previous >= signature.keyId)
      fail("Signatures must have distinct sorted key IDs");
    previous = signature.keyId;
    const raw = Buffer.from(signature.publicKey, "hex");
    if (
      !strictPointEncoding(raw) ||
      !strictPointEncoding(
        Buffer.from(signature.signature, "hex").subarray(0, 32),
      )
    )
      fail("Noncanonical/weak Ed25519 point", "G4E_INTEGRITY_FAILED");
    if (signature.keyId !== identity(hexHash(raw)))
      fail("Signer fingerprint mismatch");
    const key = createPublicKey({
      key: Buffer.concat([Buffer.from("302a300506032b6570032100", "hex"), raw]),
      format: "der",
      type: "spki",
    });
    if (
      !verify(
        null,
        signatureMessage(doc),
        key,
        Buffer.from(signature.signature, "hex"),
      )
    )
      fail("Invalid Ed25519 signature", "G4E_INTEGRITY_FAILED");
  }
  return doc;
}
export function requireG4epOrigin(doc, keyId, fail) {
  validateG4epDocument(doc, fail);
  if (!doc.authentication.signatures.some((s) => s.keyId === keyId))
    fail("Expected signer is not present", "G4E_INTEGRITY_FAILED");
  return true;
}
