import Ajv2020 from "ajv/dist/2020.js";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
const schema = JSON.parse(
  await readFile(new URL("./v1.schema.json", import.meta.url), "utf8"),
);
const fixture = JSON.parse(
  await readFile(new URL("./synthetic.g4lib", import.meta.url), "utf8"),
);
const validate = new Ajv2020({ strict: true }).compile(schema);
test("public catalog fixture keeps metadata-only games and separates base, update and DLC", () => {
  assert.equal(validate(fixture), true, JSON.stringify(validate.errors));
  assert.equal(fixture.data.preservationFiles.length, 0);
  assert.deepEqual(
    fixture.data.contents.map((c) => c.kind),
    ["version", "update", "dlc"],
  );
  assert.equal(fixture.data.contents[1].baseId, fixture.data.contents[0].id);
});
test("public catalog schema excludes private paths, personal preferences and future versions", () => {
  for (const mutate of [
    (v) => (v.schemaVersion = 2),
    (v) => (v.executeSql = "DELETE FROM files"),
    (v) => (v.data.games[0].path = "C:/private"),
    (v) =>
      v.data.preferences.push({
        gameId: "synthetic-game",
        favorite: true,
        inLibrary: true,
        visitedAt: null,
      }),
  ]) {
    const v = structuredClone(fixture);
    mutate(v);
    assert.equal(validate(v), false);
  }
});
