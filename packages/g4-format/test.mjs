import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
const read = async (p) =>
  JSON.parse(await readFile(new URL(p, import.meta.url), "utf8"));
const descriptor = await read("./fixtures/descriptor.json");
const validate = new Ajv2020({ strict: true }).compile(
  await read("./v1.schema.json"),
);
const manifest = await read("./fixtures/manifest.json");
const manifestValidate = new Ajv2020({ strict: true }).compile(
  await read("../manifest-schema/v1.schema.json"),
);
test("public g4 descriptor and embedded manifest fixtures validate offline", () => {
  assert.ok(validate(descriptor));
  assert.ok(manifestValidate(manifest));
});
test("descriptor versions, reserved paths and unknown properties are strict", () => {
  for (const bad of [
    { ...descriptor, packageVersion: 2 },
    { ...descriptor, payloadPrefix: "../" },
    { ...descriptor, manifest: "elsewhere.json" },
    { ...descriptor, extra: true },
  ])
    assert.equal(validate(bad), false);
});
