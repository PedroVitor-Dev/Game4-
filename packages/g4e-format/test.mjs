import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { test } from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import { parseG4e, G4eValidationError, MAX_BYTES } from "./validation.mjs";
const bytes = async (p) => readFile(new URL(p, import.meta.url));
const json = async (p) => JSON.parse(await bytes(p));
const original = await json("./examples/original-files.g4e");
const packaged = await json("./examples/package.g4e");
const corpus = await json("./invalid-cases.json");
const ajv = new Ajv2020({ strict: true });
ajv.addSchema(await json("../manifest-schema/v1.schema.json"));
const schema = ajv.compile(await json("./v1.schema.json"));
const encode = (doc) => JSON.stringify(doc);
const invalid = (error) => error instanceof G4eValidationError;
const digest = (algorithm, data) =>
  createHash(algorithm).update(data).digest("hex");

test("both original-file and optional-package examples validate offline", () => {
  for (const doc of [original, packaged]) {
    assert.ok(schema(doc));
    assert.deepEqual(parseG4e(encode(doc)), doc);
  }
});
test("shared malformed corpus distinguishes schema and semantic failures", () => {
  for (const item of corpus) {
    assert.equal(schema(item.document), item.schemaValid, item.name);
    assert.throws(() => parseG4e(encode(item.document)), invalid, item.name);
  }
});
test("strict JSON rejects duplicate escaped keys, malformed UTF-8, BOM and trailing input", () => {
  const valid = encode(original);
  const duplicates = valid.replace(
    '"schemaVersion":1',
    '"schemaVersion":1,"schemaVer\\u0073ion":1',
  );
  for (const text of [
    duplicates,
    valid + "{}",
    "\ufeff" + valid,
    valid.replace(
      '"extensions":{}',
      '"extensions":{"org.example.note":{"x":1,"x":2}}',
    ),
  ])
    assert.throws(() => parseG4e(text), invalid);
  assert.throws(() => parseG4e(Buffer.from([0xff])), invalid);
  const surrogate = structuredClone(original);
  surrogate.manifest.collection.title = "\ud800";
  assert.throws(() => parseG4e(encode(surrogate)), invalid);
});
test("resource limits fail before parsing oversized input or descending deeply", () => {
  assert.throws(
    () => parseG4e(Buffer.alloc(MAX_BYTES + 1)),
    (error) => error.code === "G4E_LIMIT_EXCEEDED",
  );
  assert.throws(
    () => parseG4e("[".repeat(66) + "0" + "]".repeat(66)),
    (error) => error.code === "G4E_LIMIT_EXCEEDED",
  );
});
test("magnet-only discovery requires neither a tracker nor a companion file", () => {
  const doc = structuredClone(original);
  doc.torrents[0].metainfoFile = null;
  assert.ok(parseG4e(encode(doc)));
  doc.torrents[0].metainfoFile = {
    path: "nonexistent.torrent",
    sha256: "0".repeat(64),
    sizeBytes: "1",
  };
  assert.ok(parseG4e(encode(doc))); // Descriptor validation never opens this reference.
  doc.torrents[0].magnet +=
    "&dn=Synthetic%20collection&tr=udp%3A%2F%2Ftracker.example.invalid%3A6969%2Fannounce";
  assert.ok(parseG4e(encode(doc)));
});
test("advisory namespaced metadata cannot change the core sharing representation", () => {
  const doc = structuredClone(original);
  doc.extensions = { "org.example.notes": { description: "Advisory only" } };
  assert.equal(parseG4e(encode(doc)).representation, "original_files");
  doc.manifest.files[0].path = "game/hello.txt";
  assert.throws(() => parseG4e(encode(doc)), invalid);
});
test("independently generated torrent companions bind exact info bytes and full-file SHA-256", async () => {
  for (const name of ["original-files", "package"]) {
    const doc = await json(`./examples/${name}.g4e`);
    const raw = await bytes(`./examples/${name}.torrent`);
    const ref = doc.torrents[0];
    assert.equal(raw.subarray(0, 7).toString(), "d4:info");
    assert.equal(raw.at(-1), 101);
    const info = raw.subarray(7, -1); // Fixtures contain only the top-level info key.
    assert.equal(digest("sha1", info), ref.infoHashV1);
    assert.equal(digest("sha256", raw), ref.metainfoFile.sha256);
    assert.equal(String(raw.length), ref.metainfoFile.sizeBytes);
    const data =
      name === "package"
        ? await bytes("../g4-format/fixtures/v1-minimal.g4")
        : Buffer.concat([
            Buffer.from("Preservação ∞\n"),
            Buffer.alloc(0),
            Buffer.from("Game4 package fixture\n"),
          ]);
    const piece = createHash("sha1").update(data).digest();
    assert.ok(
      info.includes(Buffer.concat([Buffer.from("6:pieces20:"), piece])),
    );
  }
  const actual = await bytes("../g4-format/fixtures/v1-minimal.g4");
  const expected = packaged.manifest.files[0];
  assert.equal(String(actual.length), expected.sizeBytes);
  assert.equal(digest("sha256", actual), expected.sha256);
});
