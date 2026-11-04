import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = async (locale) =>
  JSON.parse(
    await readFile(
      new URL(`../src/locales/${locale}.json`, import.meta.url),
      "utf8",
    ),
  );
function keys(value, prefix = "") {
  return Object.entries(value).flatMap(([key, child]) => {
    const path = prefix ? `${prefix}.${key}` : key;
    return typeof child === "object" ? keys(child, path) : [[path, child]];
  });
}
const en = new Map(keys(await read("en")));
const pt = new Map(keys(await read("pt-BR")));
assert.deepEqual(
  [...en.keys()].sort(),
  [...pt.keys()].sort(),
  "Locale keys must match",
);
for (const [key, value] of en) {
  assert.equal(typeof value, "string");
  assert.ok(value.trim() && pt.get(key).trim(), `Empty translation: ${key}`);
  const parameters = (text) =>
    [...text.matchAll(/{{(.*?)}}/g)].map((match) => match[1]).sort();
  assert.deepEqual(
    parameters(value),
    parameters(pt.get(key)),
    `Interpolation mismatch: ${key}`,
  );
}
console.log(
  `Validated ${en.size} translation keys in English and Brazilian Portuguese.`,
);
