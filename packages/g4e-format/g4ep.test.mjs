import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash, createPrivateKey, sign } from "node:crypto";
import { test } from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import { parseG4e, G4eValidationError } from "./validation.mjs";
import {
  canonicalJson,
  g4epPayloadHash,
  requireG4epOrigin,
  signatureMessage,
} from "./g4ep-validation.mjs";
const json = async (name) =>
  JSON.parse(await readFile(new URL(name, import.meta.url), "utf8"));
const unsigned = await json("./examples/g4ep-fragments-v2.g4e"),
  signed = await json("./examples/g4ep-fragments-v2-signed.g4e");
const ajv = new Ajv2020({ strict: true });
ajv.addSchema(await json("../manifest-schema/v1.schema.json"));
const structural = ajv.compile(await json("./v2.schema.json"));
const fail = (message, code = "G4E_INVALID") => {
  throw new G4eValidationError(code, message);
};
test("v2 describes all reconstruction groups without payload and validates alongside v1", async () => {
  for (const doc of [unsigned, signed]) {
    assert.ok(structural(doc), JSON.stringify(structural.errors));
    assert.deepEqual(parseG4e(JSON.stringify(doc)), doc);
    assert.equal(doc.g4ep.groups[0].fragments.length, 12);
    assert.equal(doc.contentId, `urn:sha256:${doc.manifest.files[0].sha256}`);
    assert.ok(!doc.manifest.files[0].path.includes("/"));
  }
  for (const file of [
    "./examples/original-files.g4e",
    "./examples/package.g4e",
  ])
    assert.equal(parseG4e(JSON.stringify(await json(file))).schemaVersion, 1);
});
test("shared malformed v2 corpus rejects geometry, paths, unbound peers, weak keys and tampering", async () => {
  for (const item of await json("./g4ep-invalid-cases.json"))
    assert.throws(
      () => parseG4e(JSON.stringify(item.document)),
      (e) => e instanceof G4eValidationError,
      item.name,
    );
});
test("canonical bytes and Ed25519 signatures are deterministic with explicit key pinning", () => {
  function reverse(value) {
    if (Array.isArray(value)) return value.map(reverse);
    if (value && typeof value === "object")
      return Object.fromEntries(
        Object.entries(value)
          .reverse()
          .map(([k, v]) => [k, reverse(v)]),
      );
    return value;
  }
  const doc = reverse(signed);
  assert.equal(canonicalJson(doc), canonicalJson(signed));
  assert.equal(g4epPayloadHash(doc), signed.authentication.payloadSha256);
  assert.deepEqual(parseG4e(JSON.stringify(doc)), signed);
  assert.equal(
    requireG4epOrigin(signed, signed.authentication.signatures[0].keyId, fail),
    true,
  );
  assert.throws(() =>
    requireG4epOrigin(signed, "urn:sha256:" + "0".repeat(64), fail),
  );
  assert.throws(() =>
    requireG4epOrigin(
      unsigned,
      signed.authentication.signatures[0].keyId,
      fail,
    ),
  );
  const key = createPrivateKey({
    key: Buffer.from(
      "302e020100300506032b6570042204209d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
      "hex",
    ),
    format: "der",
    type: "pkcs8",
  });
  assert.equal(
    sign(null, signatureMessage(signed), key).toString("hex"),
    signed.authentication.signatures[0].signature,
  );
});
test("duplicate keys, payload injection and noncanonical lengths never become valid v2", () => {
  const text = JSON.stringify(unsigned);
  for (const token of ["2.0", "2e0", "-0"]) {
    assert.throws(() =>
      parseG4e(text.replace('"schemaVersion":2', '"schemaVersion":' + token)),
    );
  }
  assert.throws(() =>
    parseG4e(
      text.replace(
        '"schemaVersion":2',
        '"schemaVersion":2,"schemaVer\\u0073ion":2',
      ),
    ),
  );
  for (const modify of [
    (d) => (d.romBytes = "payload"),
    (d) => (d.g4ep.groups[0].fragments[0].sizeBytes = "065536"),
    (d) => (d.manifest.files[0].path = "\\\\host\\private.rom"),
  ]) {
    const d = structuredClone(unsigned);
    modify(d);
    d.authentication.payloadSha256 = g4epPayloadHash(d);
    assert.throws(() => parseG4e(JSON.stringify(d)));
  }
});
test("self-consistent empty, padded and multi-group descriptors preserve exact coverage", () => {
  const empty = structuredClone(unsigned);
  empty.manifest.files[0].sizeBytes = "0";
  empty.manifest.files[0].sha256 = createHash("sha256")
    .update("")
    .digest("hex");
  empty.contentId = `urn:sha256:${empty.manifest.files[0].sha256}`;
  empty.g4ep.groups = [];
  empty.discovery = [];
  empty.authentication.payloadSha256 = g4epPayloadHash(empty);
  assert.ok(parseG4e(JSON.stringify(empty)));
  const padded = structuredClone(unsigned);
  padded.manifest.files[0].sizeBytes = "1";
  padded.g4ep.groups[0].originalBytes = "1";
  padded.authentication.payloadSha256 = g4epPayloadHash(padded);
  assert.ok(parseG4e(JSON.stringify(padded)));
  const multi = structuredClone(unsigned);
  multi.manifest.files[0].sizeBytes = "524289";
  multi.g4ep.groups.push({
    ...structuredClone(multi.g4ep.groups[0]),
    id: "g000000000001",
    index: "1",
    originalOffset: "524288",
    originalBytes: "1",
  });
  multi.authentication.payloadSha256 = g4epPayloadHash(multi);
  assert.ok(parseG4e(JSON.stringify(multi)));
});
