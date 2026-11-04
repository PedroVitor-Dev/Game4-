import assert from "node:assert/strict";
import { test } from "node:test";
import { readFile } from "node:fs/promises";
import { createIpcClient } from "../src/services/ipc-client.ts";
import {
  ERROR_CODES,
  IpcFailure,
  normalizeIpcError,
  parseAppInfo,
  parseDatabaseInfo,
} from "@game4/ipc-contracts";
import { getIpcErrorKey } from "../src/services/errors.ts";

const examples = JSON.parse(
  await readFile(
    new URL("../../../packages/ipc-contracts/examples.json", import.meta.url),
    "utf8",
  ),
);
const make = (invoke, available = true) =>
  createIpcClient({ invoke, isAvailable: () => available });
const code = (expected) => (error) =>
  error instanceof IpcFailure && error.error.code === expected;

test("Rust-serialized examples validate and unknown additive fields are ignored", () => {
  assert.deepEqual(
    parseAppInfo({ ...examples.appInfo, future: "ignored" }),
    examples.appInfo,
  );
  assert.deepEqual(
    parseDatabaseInfo(examples.databaseInfo),
    examples.databaseInfo,
  );
  assert.deepEqual(normalizeIpcError(examples.error), examples.error);
});

test("concurrent reads share the handshake and then use only advertised commands", async () => {
  const calls = [];
  const client = make(async (command) => {
    calls.push(command);
    return command === "app_info" ? examples.appInfo : examples.databaseInfo;
  });
  await Promise.all([client.getDatabaseInfo(), client.getDatabaseInfo()]);
  assert.deepEqual(calls, ["app_info", "database_info", "database_info"]);
});

test("browser mode never invokes a native command", async () => {
  const client = make(async () => {
    assert.fail("must not invoke");
  }, false);
  await assert.rejects(client.getDatabaseInfo(), code("IPC_UNAVAILABLE"));
});

test("version mismatch and missing capabilities prevent database invocation", async () => {
  for (const [info, expected] of [
    [{ ...examples.appInfo, contractVersion: 2 }, "CONTRACT_MISMATCH"],
    [{ ...examples.appInfo, capabilities: [] }, "CAPABILITY_UNAVAILABLE"],
  ]) {
    const client = make(async (command) => {
      assert.equal(command, "app_info");
      return info;
    });
    await assert.rejects(client.getDatabaseInfo(), code(expected));
  }
});

test("failed handshakes can be retried without caching a rejection forever", async () => {
  let calls = 0;
  const client = make(async () => {
    if (++calls === 1) throw examples.error;
    return examples.appInfo;
  });
  await assert.rejects(client.getAppInfo(), code("DATABASE_UNAVAILABLE"));
  assert.deepEqual(await client.getAppInfo(), examples.appInfo);
});

test("invalid numeric values and response shapes cannot reach consumers", () => {
  for (const value of [
    null,
    [],
    {},
    { ...examples.databaseInfo, fileCount: -1 },
    { ...examples.databaseInfo, fileCount: 1.5 },
    { ...examples.databaseInfo, fileCount: Number.MAX_SAFE_INTEGER + 1 },
    { ...examples.databaseInfo, fileCount: "3" },
    { ...examples.databaseInfo, schemaVersion: 0 },
  ]) {
    assert.throws(() => parseDatabaseInfo(value), code("INVALID_RESPONSE"));
  }
});

test("unstructured native errors are sanitized and all known errors have both translations", async () => {
  const normalized = normalizeIpcError(
    "private database path must never reach the UI",
  );
  assert.deepEqual(normalized, {
    code: "UNEXPECTED_ERROR",
    retryable: false,
    field: null,
  });
  for (const locale of ["en", "pt-BR"]) {
    const strings = JSON.parse(
      await readFile(
        new URL(`../src/locales/${locale}.json`, import.meta.url),
        "utf8",
      ),
    );
    for (const errorCode of ERROR_CODES) {
      const key = getIpcErrorKey({
        code: errorCode,
        retryable: false,
        field: null,
      }).split(".")[1];
      assert.ok(strings.errors[key]?.trim());
    }
  }
});

test("library requests are wrapped and responses validated at the boundary", async () => {
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "library_list")
      return {
        items: [
          {
            id: "1",
            title: "Test",
            platform: null,
            fileCount: 1,
            totalBytes: "9007199254740993",
          },
        ],
        nextCursor: null,
      };
    return { cancelled: true, selectionToken: null, displayNames: [] };
  });
  const request = { page: { cursor: null, limit: 50 }, query: null };
  const page = await client.listLibrary(request);
  assert.equal(page.items[0].totalBytes, "9007199254740993");
  assert.deepEqual(calls[1], ["library_list", { request }]);
  assert.equal((await client.selectSources({ kind: "files" })).cancelled, true);
  const invalid = make(async (command) =>
    command === "app_info"
      ? examples.appInfo
      : { items: [{ id: 1 }], nextCursor: null },
  );
  await assert.rejects(invalid.listLibrary(request), code("INVALID_RESPONSE"));
});
test("task runtime decoders reject malformed state and progress", async () => {
  const { parseTaskSnapshot, parseSourceSelection } =
    await import("@game4/ipc-contracts");
  const task = examples.draftTaskEvent.task;
  assert.deepEqual(parseTaskSnapshot(task), task);
  for (const patch of [
    { status: "unknown" },
    { processedCount: "6" },
    { revision: "01" },
    { updatedAt: "yesterday" },
    { status: "failed", error: null },
    { kind: "torrent" },
  ])
    assert.throws(
      () => parseTaskSnapshot({ ...task, ...patch }),
      code("INVALID_RESPONSE"),
    );
  assert.throws(
    () =>
      parseSourceSelection({
        cancelled: false,
        selectionToken: null,
        displayNames: [],
      }),
    code("INVALID_RESPONSE"),
  );
});
