import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
const read = async (name) =>
  JSON.parse(await readFile(new URL(name, import.meta.url), "utf8"));
const schema = await read("./v1.schema.json");
const example = await read("./examples/v1.example.json");
const cases = await read("./invalid-cases.json");
const validate = new Ajv2020({ strict: true, allErrors: true }).compile(schema);
test("manifest v1 example validates offline against the public schema", () => {
  assert.ok(validate(example), JSON.stringify(validate.errors));
});
test("shared malformed corpus distinguishes schema and cross-file semantic rules", () => {
  for (const c of cases)
    assert.equal(
      validate(c.manifest),
      c.schemaValid,
      c.name + JSON.stringify(validate.errors),
    );
});
test("exact large sizes, Unicode paths, and namespaced extensions are structurally supported", () => {
  const value = structuredClone(example);
  value.files[0].sizeBytes = "9007199254740993";
  value.files[0].path = "Coleção ∞/日本語.bin";
  value.extensions = { "org.example.metadata": { declared: true } };
  assert.ok(validate(value), JSON.stringify(validate.errors));
});
