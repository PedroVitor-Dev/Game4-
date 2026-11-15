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

test("manifest export uses item IDs and validates cancellation and filenames", async () => {
  const { parseExportManifestResult } = await import("@game4/ipc-contracts");
  assert.deepEqual(
    parseExportManifestResult({ cancelled: true, fileName: null }),
    { cancelled: true, fileName: null },
  );
  for (const v of [
    { cancelled: true, fileName: "x.json" },
    { cancelled: false, fileName: null },
    { cancelled: false, fileName: "C:/private/x.json" },
  ])
    assert.throws(() => parseExportManifestResult(v), code("INVALID_RESPONSE"));
  const client = make(async (command, args) => {
    if (command === "app_info") return examples.appInfo;
    assert.equal(command, "library_export_manifest");
    assert.deepEqual(args, { request: { itemId: "1" } });
    return { cancelled: false, fileName: "manifest.v1.json" };
  });
  assert.equal(
    (await client.exportManifest({ itemId: "1" })).fileName,
    "manifest.v1.json",
  );
});

test("library backup and restore use preview tokens, validate counts, and never accept paths", async () => {
  const { parseLibraryPrepareRestoreResult, parseLibraryRestoreResult } =
    await import("@game4/ipc-contracts");
  const preview = {
    restoreToken: "12345678-1234-4234-9234-123456789abc",
    fileName: "library.json",
    itemCount: 1,
    fileCount: 2,
    locationCount: 3,
    taskCount: 4,
  };
  assert.deepEqual(
    parseLibraryPrepareRestoreResult({ cancelled: false, preview }),
    { cancelled: false, preview },
  );
  assert.deepEqual(
    parseLibraryPrepareRestoreResult({ cancelled: true, preview: null }),
    { cancelled: true, preview: null },
  );
  for (const p of [
    { ...preview, fileName: "C:/private.json" },
    { ...preview, itemCount: -1 },
    { ...preview, restoreToken: "arbitrary-path" },
    { ...preview, fileCount: 500001 },
  ])
    assert.throws(
      () => parseLibraryPrepareRestoreResult({ cancelled: false, preview: p }),
      code("INVALID_RESPONSE"),
    );
  assert.throws(
    () =>
      parseLibraryRestoreResult({
        ...preview,
        recoveryFileName: "../recovery.json",
      }),
    code("INVALID_RESPONSE"),
  );
  const client = make(async (command, args) => {
    if (command === "app_info") return examples.appInfo;
    if (command === "library_save_backup") {
      assert.equal(args, undefined);
      return { cancelled: false, fileName: "library.json" };
    }
    if (command === "library_prepare_restore") {
      assert.equal(args, undefined);
      return { cancelled: false, preview };
    }
    assert.equal(command, "library_restore_backup");
    assert.deepEqual(args, { request: { restoreToken: preview.restoreToken } });
    return {
      itemCount: 1,
      fileCount: 2,
      locationCount: 3,
      taskCount: 4,
      recoveryFileName: "recovery.json",
    };
  });
  assert.equal((await client.saveLibraryBackup()).fileName, "library.json");
  assert.equal(
    (await client.prepareLibraryRestore()).preview.restoreToken,
    preview.restoreToken,
  );
  assert.equal(
    (await client.restoreLibraryBackup({ restoreToken: preview.restoreToken }))
      .recoveryFileName,
    "recovery.json",
  );
});

test("readable IEC sizes preserve exact integers and localize decimal separators", async () => {
  const { formatBytes } =
    await import("../src/services/file-identification.ts");
  for (const [value, en, pt] of [
    ["0", "0 B", "0 B"],
    ["1023", "1,023 B", "1.023 B"],
    ["1024", "1 KiB", "1 KiB"],
    ["1536", "1.5 KiB", "1,5 KiB"],
    ["1048576", "1 MiB", "1 MiB"],
    ["9007199254740993", "8 PiB", "8 PiB"],
    ["18446744073709551614", "16 EiB", "16 EiB"],
  ]) {
    assert.equal(formatBytes(value, "en"), en);
    assert.equal(formatBytes(value, "pt-BR"), pt);
  }
  for (const invalid of ["-1", "01", "NaN", "1.5"])
    assert.throws(() => formatBytes(invalid, "en"), RangeError);
});
test("file identification responses validate vocabulary, evidence and precise sizes", async () => {
  const {
    parseLibraryFileInfo,
    parseLibraryItemFilesPage,
    FILE_FORMATS,
    PLATFORM_IDS,
  } = await import("@game4/ipc-contracts");
  const { formatLabels, platformLabels } =
    await import("../src/services/file-identification.ts");
  for (const id of FILE_FORMATS) assert.ok(formatLabels[id]);
  for (const id of PLATFORM_IDS) assert.ok(platformLabels[id]);
  const file = {
    path: "日本語.nes",
    sizeBytes: "9007199254740993",
    sha256: "a".repeat(64),
    format: "ines",
    formatEvidence: "signature",
    platform: "nes",
    platformEvidence: "signature",
    detectorVersion: 1,
  };
  assert.deepEqual(parseLibraryFileInfo(file), file);
  for (const patch of [
    { format: "invented" },
    { sizeBytes: 3 },
    { sha256: "x" },
    { detectorVersion: null },
    { platform: null },
    { format: "unknown" },
  ])
    assert.throws(
      () => parseLibraryFileInfo({ ...file, ...patch }),
      code("INVALID_RESPONSE"),
    );
  assert.throws(
    () => parseLibraryItemFilesPage({ files: [], nextCursor: "cursor" }),
    code("INVALID_RESPONSE"),
  );
  const client = make(async (command, args) => {
    if (command === "app_info") return examples.appInfo;
    assert.equal(command, "library_item_files");
    assert.deepEqual(args, {
      request: { itemId: "1", page: { cursor: null, limit: 50 } },
    });
    return { files: [file], nextCursor: null };
  });
  assert.equal(
    (
      await client.getItemFiles({
        itemId: "1",
        page: { cursor: null, limit: 50 },
      })
    ).files[0].sizeBytes,
    file.sizeBytes,
  );
});

test("manual verification uses advertised commands and validates copy observations", async () => {
  const request = {
    requestId: "12345678-1234-4234-9234-123456789abc",
    itemId: "1",
  };
  const health = {
    verified: 2,
    changed: 1,
    missing: 1,
    inaccessible: 0,
    unverified: 0,
    unlocated: 1,
    lastCheckedAt: "2026-11-13T12:00:00.000Z",
  };
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "library_verify") return { taskId: "8", status: "queued" };
    return health;
  });
  assert.deepEqual(await client.verifyItem(request), {
    taskId: "8",
    status: "queued",
  });
  assert.deepEqual(calls[1], ["library_verify", { request }]);
  assert.deepEqual(await client.getIntegrity({ itemId: "1" }), health);
  for (const bad of [
    { ...health, changed: -1 },
    { ...health, verified: "2" },
    { ...health, lastCheckedAt: "invalid" },
    { ...health, unlocated: undefined },
  ]) {
    const invalid = make(async (command) =>
      command === "app_info" ? examples.appInfo : bad,
    );
    await assert.rejects(
      invalid.getIntegrity({ itemId: "1" }),
      code("INVALID_RESPONSE"),
    );
  }
  const old = make(async () => ({ ...examples.appInfo, capabilities: [] }));
  await assert.rejects(old.verifyItem(request), code("CAPABILITY_UNAVAILABLE"));
  await assert.rejects(
    make(async () => assert.fail("native call"), false).verifyItem(request),
    code("IPC_UNAVAILABLE"),
  );
});

test("g4 export and reading are capability gated and validate verified package summaries", async () => {
  const summary = {
    fileName: "fixture.g4",
    packageVersion: 1,
    collectionId: "12345678-1234-4234-9234-123456789abc",
    title: "Synthetic fixture",
    fileCount: 3,
    totalBytes: "40",
    verified: true,
  };
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "library_export_package")
      return { cancelled: false, fileName: "fixture.g4" };
    return { cancelled: false, package: summary };
  });
  assert.deepEqual(await client.exportPackage({ itemId: "1" }), {
    cancelled: false,
    fileName: "fixture.g4",
  });
  assert.deepEqual(calls[1], [
    "library_export_package",
    { request: { itemId: "1" } },
  ]);
  assert.deepEqual(await client.openPackage(), {
    cancelled: false,
    package: summary,
  });
  assert.deepEqual(calls[2], ["package_open", undefined]);
  for (const bad of [
    { ...summary, verified: false },
    { ...summary, fileCount: 100001 },
    { ...summary, totalBytes: "1099511627777" },
    { ...summary, packageVersion: 2 },
    { ...summary, fileName: "../bad.g4" },
    { ...summary, collectionId: "not-a-uuid" },
  ]) {
    const invalid = make(async (command) =>
      command === "app_info"
        ? examples.appInfo
        : { cancelled: false, package: bad },
    );
    await assert.rejects(invalid.openPackage(), code("INVALID_RESPONSE"));
  }
  const cancelled = make(async (command) =>
    command === "app_info"
      ? examples.appInfo
      : { cancelled: true, package: null },
  );
  assert.deepEqual(await cancelled.openPackage(), {
    cancelled: true,
    package: null,
  });
  const old = make(async () => ({ ...examples.appInfo, capabilities: [] }));
  await assert.rejects(old.openPackage(), code("CAPABILITY_UNAVAILABLE"));
  await assert.rejects(
    make(async () => assert.fail("native call"), false).exportPackage({
      itemId: "1",
    }),
    code("IPC_UNAVAILABLE"),
  );
});

test("ever export and read use Rust DTOs, validate reference-only summaries and require capabilities", async () => {
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    return command === "app_info"
      ? examples.appInfo
      : command === "ever_open"
        ? examples.everOpen
        : examples.everExport;
  });
  assert.deepEqual(
    await client.exportEver({ itemId: "1" }),
    examples.everExport,
  );
  assert.deepEqual(calls[1], [
    "library_export_ever",
    { request: { itemId: "1" } },
  ]);
  assert.deepEqual(await client.openEver(), examples.everOpen);
  assert.deepEqual(calls[2], ["ever_open", undefined]);
  for (const field of [
    { contentVerified: true },
    { schemaVersion: 2 },
    { magnets: ["javascript:alert(1)"] },
    { fileCount: 0 },
    { representation: "executable" },
    { totalBytes: "18446744073709551616" },
  ]) {
    const bad = {
      ...examples.everOpen,
      manifest: { ...examples.everOpen.manifest, ...field },
    };
    await assert.rejects(
      make(async (command) =>
        command === "app_info" ? examples.appInfo : bad,
      ).openEver(),
      code("INVALID_RESPONSE"),
    );
  }
  const cancelled = make(async (command) =>
    command === "app_info"
      ? examples.appInfo
      : { cancelled: true, manifest: null },
  );
  assert.deepEqual(await cancelled.openEver(), {
    cancelled: true,
    manifest: null,
  });
  const old = make(async () => ({ ...examples.appInfo, capabilities: [] }));
  await assert.rejects(
    old.exportEver({ itemId: "1" }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    make(async () => assert.fail("native invocation"), false).openEver(),
    code("IPC_UNAVAILABLE"),
  );
});

test("torrent IPC validates previews, exact progress, cancellation and capabilities", async () => {
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "torrent_import") return examples.torrentImport;
    if (command === "transfers_list") return { items: [examples.transfer] };
    if (command === "torrent_download")
      return { cancelled: false, transfer: examples.transfer };
    return examples.transfer;
  });
  assert.deepEqual(await client.importTorrent(), examples.torrentImport);
  const request = {
    requestId: examples.transfer.id,
    token: examples.torrentImport.preview.token,
    peer: "127.0.0.1:42420",
  };
  assert.equal(
    (await client.downloadTorrent(request)).transfer.id,
    examples.transfer.id,
  );
  assert.deepEqual(calls[2], ["torrent_download", { request }]);
  assert.equal((await client.listTransfers()).items.length, 1);
  assert.deepEqual(
    await client.stopTransfer({ transferId: examples.transfer.id }),
    examples.transfer,
  );
  for (const change of [
    { verifiedBytes: "41" },
    { totalBytes: "0" },
    { status: "running" },
    { listenPort: 65536 },
    { error: { code: "TRANSFER_FAILED", field: null } },
    { outputName: null },
  ]) {
    await assert.rejects(
      make(async (command) =>
        command === "app_info"
          ? examples.appInfo
          : { items: [{ ...examples.transfer, ...change }] },
      ).listTransfers(),
      code("INVALID_RESPONSE"),
    );
  }
  for (const path of ["../escape", "/absolute", "C:/escape"]) {
    const preview = {
      ...examples.torrentImport.preview,
      files: [{ path, sizeBytes: "40" }],
    };
    await assert.rejects(
      make(async (command) =>
        command === "app_info"
          ? examples.appInfo
          : { cancelled: false, preview },
      ).importTorrent(),
      code("INVALID_RESPONSE"),
    );
  }
  assert.deepEqual(
    await make(async (command) =>
      command === "app_info"
        ? examples.appInfo
        : { cancelled: true, transfer: null },
    ).downloadTorrent(request),
    { cancelled: true, transfer: null },
  );
  await assert.rejects(
    make(async () => ({
      ...examples.appInfo,
      capabilities: [],
    })).importTorrent(),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    make(async () => assert.fail("native invocation"), false).listTransfers(),
    code("IPC_UNAVAILABLE"),
  );
});

test("magnet commands are capability gated, wrap requests and bind generated identity", async () => {
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "magnet_generate") return examples.magnetLink;
    if (command === "magnet_cancel") return { cancelled: true };
    return examples.torrentImport;
  });
  assert.deepEqual(
    await client.generateMagnet({ itemId: "1" }),
    examples.magnetLink,
  );
  const request = {
    requestId: examples.transfer.id,
    magnet: examples.magnetLink.magnet,
    peer: "",
    dht: true,
  };
  assert.deepEqual(await client.resolveMagnet(request), examples.torrentImport);
  assert.deepEqual(calls[2], ["magnet_resolve", { request }]);
  assert.deepEqual(
    await client.cancelMagnet({ requestId: request.requestId }),
    { cancelled: true },
  );
  assert.deepEqual(calls[3], [
    "magnet_cancel",
    { request: { requestId: request.requestId } },
  ]);
  for (const result of [
    { ...examples.magnetLink, infoHash: "0".repeat(40) },
    { ...examples.magnetLink, magnet: "https://invalid.example/" },
    { ...examples.magnetLink, infoHash: "invalid" },
  ]) {
    await assert.rejects(
      make(async (command) =>
        command === "app_info" ? examples.appInfo : result,
      ).generateMagnet({ itemId: "1" }),
      code("INVALID_RESPONSE"),
    );
  }
  await assert.rejects(
    make(async (command) =>
      command === "app_info" ? examples.appInfo : { cancelled: "true" },
    ).cancelMagnet({ requestId: request.requestId }),
    code("INVALID_RESPONSE"),
  );
  await assert.rejects(
    make(async (command) =>
      command === "app_info"
        ? examples.appInfo
        : { items: [{ ...examples.transfer, dht: "true" }] },
    ).listTransfers(),
    code("INVALID_RESPONSE"),
  );
  const old = make(async () => ({ ...examples.appInfo, capabilities: [] }));
  await assert.rejects(
    old.resolveMagnet(request),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.generateMagnet({ itemId: "1" }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.cancelMagnet({ requestId: request.requestId }),
    code("CAPABILITY_UNAVAILABLE"),
  );
});

test("DHT requests cannot be silently ignored by an older v1 backend", async () => {
  const capabilities = examples.appInfo.capabilities.filter(
    (c) => !c.startsWith("magnet_"),
  );
  const client = make(async (command) =>
    command === "app_info"
      ? { ...examples.appInfo, capabilities }
      : examples.transfer,
  );
  await assert.rejects(
    client.seedTorrent({
      requestId: examples.transfer.id,
      itemId: "1",
      listenPort: 0,
      dht: true,
    }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    client.downloadTorrent({
      requestId: examples.transfer.id,
      token: examples.transfer.id,
      peer: "",
      dht: true,
    }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  const { dht, ...legacy } = examples.transfer;
  assert.equal(
    (
      await make(async (command) =>
        command === "app_info" ? examples.appInfo : legacy,
      ).seedTorrent({
        requestId: examples.transfer.id,
        itemId: "1",
        listenPort: 0,
        dht: false,
      })
    ).dht,
    false,
  );
});

test("tracker controls validate endpoints, gate older backends and preserve optional v1 fields", async () => {
  const trackers = [
    "https://tracker.example/announce?key=private",
    "udp://tracker.example:6969/announce",
  ];
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "trackers_validate") return { trackers };
    if (command === "magnet_generate")
      return {
        ...examples.magnetLink,
        magnet:
          examples.magnetLink.magnet +
          "&tr=" +
          encodeURIComponent(trackers[0]) +
          "&tr=" +
          encodeURIComponent(trackers[1]),
      };
    if (command === "torrent_import")
      return {
        ...examples.torrentImport,
        preview: { ...examples.torrentImport.preview, trackers },
      };
    return { ...examples.transfer, trackerCount: 2 };
  });
  assert.deepEqual(await client.validateTrackers({ trackers }), { trackers });
  const result = await client.generateMagnet({ itemId: "1", trackers });
  assert.equal(new URL(result.magnet).searchParams.getAll("tr").length, 2);
  assert.deepEqual(calls[2], ["trackers_validate", { request: { trackers } }]);
  assert.deepEqual((await client.importTorrent()).preview.trackers, trackers);
  assert.equal(
    (
      await client.seedTorrent({
        requestId: examples.transfer.id,
        itemId: "1",
        listenPort: 0,
        dht: false,
        trackers,
      })
    ).trackerCount,
    2,
  );
  for (const value of [
    "file:///tmp/a",
    "https://user:secret@tracker.example/",
    "udp://tracker.example/announce",
    "http://tracker.example/#x",
    "ws://tracker.example/",
    "http://tracker.example:0/",
  ]) {
    await assert.rejects(
      make(async (command) =>
        command === "app_info" ? examples.appInfo : { trackers: [value] },
      ).validateTrackers({ trackers: [] }),
      code("INVALID_RESPONSE"),
    );
  }
  await assert.rejects(
    make(async (command) =>
      command === "app_info"
        ? examples.appInfo
        : { trackers: Array(17).fill(trackers[0]) },
    ).validateTrackers({ trackers: [] }),
    code("INVALID_RESPONSE"),
  );
  await assert.rejects(
    make(async (command) =>
      command === "app_info"
        ? examples.appInfo
        : { items: [{ ...examples.transfer, trackerCount: 17 }] },
    ).listTransfers(),
    code("INVALID_RESPONSE"),
  );
  const old = make(async () => ({
    ...examples.appInfo,
    capabilities: examples.appInfo.capabilities.filter(
      (c) => c !== "trackers_validate",
    ),
  }));
  await assert.rejects(
    old.generateMagnet({ itemId: "1", trackers }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.downloadTorrent({
      requestId: examples.transfer.id,
      token: examples.transfer.id,
      peer: "",
      dht: false,
      trackers,
    }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  const { trackerCount, ...legacy } = examples.transfer;
  assert.equal(
    (
      await make(async (command) =>
        command === "app_info" ? examples.appInfo : { items: [legacy] },
      ).listTransfers()
    ).items[0].trackerCount,
    0,
  );
});

test("live transfer observations validate peer counts, rates and explicit continued seeding", async () => {
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "transfer_metrics_info")
      return { connectedPeerMetrics: true, seedAfterDownload: true };
    if (command === "torrent_download")
      return { cancelled: false, transfer: examples.activeTransfer };
    return { items: [examples.activeTransfer] };
  });
  assert.deepEqual(
    (await client.listTransfers()).items[0].telemetry,
    examples.activeTransfer.telemetry,
  );
  const request = {
    requestId: examples.transfer.id,
    token: examples.transfer.id,
    peer: "127.0.0.1:42420",
    dht: false,
    trackers: [],
    seedAfterDownload: true,
  };
  assert.equal(
    (await client.downloadTorrent(request)).transfer.seedAfterDownload,
    true,
  );
  assert.deepEqual(calls[2], ["transfer_metrics_info", undefined]);
  assert.deepEqual(calls[3], ["torrent_download", { request }]);
  const base = examples.activeTransfer;
  for (const change of [
    { telemetry: { ...base.telemetry, seeders: 4 } },
    { telemetry: { ...base.telemetry, connectedPeers: -1 } },
    { telemetry: { ...base.telemetry, downloadBps: "NaN" } },
    { telemetry: { ...base.telemetry, uploadBps: "18446744073709551616" } },
    { status: "stopped" },
    { isSeeding: true },
    { seedAfterDownload: "true" },
    { receivedBytes: "-1" },
  ])
    await assert.rejects(
      make(async (c) =>
        c === "app_info"
          ? examples.appInfo
          : { items: [{ ...base, ...change }] },
      ).listTransfers(),
      code("INVALID_RESPONSE"),
    );
  const seeding = { ...base, verifiedBytes: base.totalBytes, isSeeding: true };
  assert.equal(
    (
      await make(async (c) =>
        c === "app_info" ? examples.appInfo : { items: [seeding] },
      ).listTransfers()
    ).items[0].isSeeding,
    true,
  );
  const { telemetry, receivedBytes, seedAfterDownload, isSeeding, ...legacy } =
    examples.transfer;
  const oldResult = (
    await make(async (c) =>
      c === "app_info" ? examples.appInfo : { items: [legacy] },
    ).listTransfers()
  ).items[0];
  assert.equal(oldResult.telemetry, null);
  assert.equal(oldResult.isSeeding, false);
  const old = make(async () => ({
    ...examples.appInfo,
    capabilities: examples.appInfo.capabilities.filter(
      (c) => c !== "transfer_metrics_info",
    ),
  }));
  await assert.rejects(
    old.downloadTorrent(request),
    code("CAPABILITY_UNAVAILABLE"),
  );
});

test("pause/resume/history validate states, gate capabilities and never expose stale live observations", async () => {
  const paused = {
    ...examples.activeTransfer,
    status: "paused",
    isSeeding: false,
    telemetry: null,
    listenPort: null,
  };
  const calls = [];
  const client = make(async (command, args) => {
    calls.push([command, args]);
    if (command === "app_info") return examples.appInfo;
    if (command === "transfer_forget") return { forgotten: true };
    return command === "transfer_resume" ? examples.activeTransfer : paused;
  });
  const request = { transferId: examples.transfer.id };
  assert.equal((await client.pauseTransfer(request)).status, "paused");
  assert.equal((await client.resumeTransfer(request)).status, "active");
  assert.deepEqual(await client.forgetTransfer(request), { forgotten: true });
  assert.deepEqual(calls.slice(1), [
    ["transfer_pause", { request }],
    ["transfer_resume", { request }],
    ["transfer_forget", { request }],
  ]);
  for (const change of [
    { telemetry: examples.activeTransfer.telemetry },
    { isSeeding: true },
  ])
    await assert.rejects(
      make(async (c) =>
        c === "app_info"
          ? examples.appInfo
          : { items: [{ ...paused, ...change }] },
      ).listTransfers(),
      code("INVALID_RESPONSE"),
    );
  assert.equal(
    (
      await make(async (c) =>
        c === "app_info"
          ? examples.appInfo
          : { items: [{ ...paused, status: "resuming" }] },
      ).listTransfers()
    ).items[0].status,
    "resuming",
  );
  const old = make(async () => ({ ...examples.appInfo, capabilities: [] }));
  for (const method of ["pauseTransfer", "resumeTransfer", "forgetTransfer"])
    await assert.rejects(old[method](request), code("CAPABILITY_UNAVAILABLE"));
});

// Progress must stay exact for large payloads and never round unfinished data to 100%.
const { verifiedPercent, observedTelemetry } =
  await import("../src/services/transfer-metrics.ts");
test("verified transfer percentages handle partial, complete, zero and large byte totals", () => {
  assert.equal(verifiedPercent("0", "0"), 0);
  assert.equal(verifiedPercent("1", "3"), 33.3);
  assert.equal(
    verifiedPercent("999999999999999999", "1000000000000000000"),
    99.9,
  );
  assert.equal(
    verifiedPercent("1000000000000000000", "1000000000000000000"),
    100,
  );
  assert.equal(verifiedPercent("12", "10"), 100);
});
test("live observations disappear on polling failure and all inactive transfer states", () => {
  const telemetry = {
    downloadBps: "0",
    uploadBps: "512",
    connectedPeers: 2,
    seeders: 1,
    leechers: 1,
  };
  assert.equal(
    observedTelemetry({ status: "active", telemetry }, true),
    telemetry,
  );
  assert.equal(observedTelemetry({ status: "active", telemetry }, false), null);
  for (const status of [
    "paused",
    "resuming",
    "stopping",
    "stopped",
    "failed",
    "completed",
  ])
    assert.equal(observedTelemetry({ status, telemetry }, true), null);
  assert.equal(
    observedTelemetry({ status: "active", telemetry: null }, true),
    null,
  );
});

const { parseBandwidthInput } =
  await import("../src/services/transfer-metrics.ts");
const { parseBandwidthLimits } = await import("@game4/ipc-contracts");
test("bandwidth inputs convert KiB/s exactly and reject unsafe engine limits", () => {
  assert.equal(parseBandwidthInput("0"), 0);
  assert.equal(parseBandwidthInput("16"), 16384);
  assert.equal(parseBandwidthInput("128.5"), 131584);
  assert.equal(parseBandwidthInput("4194303.9990234375"), 4294967295);
  for (const value of [
    "",
    "-1",
    "1",
    "15.9",
    "NaN",
    "Infinity",
    "1e6",
    "4194304",
    "16.0001",
  ])
    assert.equal(parseBandwidthInput(value), null, value);
  assert.deepEqual(parseBandwidthLimits({ downloadBps: 0, uploadBps: 16384 }), {
    downloadBps: 0,
    uploadBps: 16384,
  });
  for (const value of [-1, 1, 16383, 4294967296, 16384.5, "65536"])
    assert.throws(
      () => parseBandwidthLimits({ downloadBps: value, uploadBps: 0 }),
      code("INVALID_RESPONSE"),
    );
});
test("bandwidth commands are capability gated and older starts cannot silently ignore a cap", async () => {
  const limits = { downloadBps: 65536, uploadBps: 131072 };
  const calls = [];
  const old = make(async (command) => {
    calls.push(command);
    if (command === "app_info")
      return {
        ...examples.appInfo,
        capabilities: examples.appInfo.capabilities.filter(
          (c) => c !== "transfer_set_limits",
        ),
      };
    throw new Error("must not start");
  });
  await assert.rejects(
    old.seedTorrent({ limits, dht: false, trackers: [] }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.downloadTorrent({
      limits,
      dht: false,
      trackers: [],
      seedAfterDownload: false,
    }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.setTransferLimits({ transferId: examples.transfer.id, limits }),
    code("CAPABILITY_UNAVAILABLE"),
  );
  assert.deepEqual(calls, ["app_info"]);
  const client = make(async (command, args) => {
    if (command === "app_info") return examples.appInfo;
    assert.equal(command, "transfer_set_limits");
    assert.deepEqual(args.request.limits, limits);
    return { ...examples.transfer, limits };
  });
  const result = await client.setTransferLimits({
    transferId: examples.transfer.id,
    limits,
  });
  assert.deepEqual(result.limits, limits);
  await assert.rejects(
    client.setTransferLimits({
      transferId: result.id,
      limits: { ...limits, uploadBps: 1 },
    }),
    code("INVALID_ARGUMENT"),
  );
});

const { transferView, sourceChoices } =
  await import("../src/services/navigation.ts");
test("download, share and monitoring routes lead to distinct transfer views", () => {
  assert.equal(transferView("download"), "download");
  assert.equal(transferView("share"), "seed");
  assert.equal(transferView("transfers"), "sessions");
  for (const page of ["dashboard", "library", "community", "tasks", "settings"])
    assert.equal(transferView(page), null);
});
test("sharing keeps the selected collection even outside the first source page", () => {
  const first = { id: "1", title: "First collection" };
  const selected = { id: "300", title: "Selected collection" };
  assert.deepEqual(sourceChoices([first], selected), [selected, first]);
  assert.deepEqual(sourceChoices([first, selected], selected), [
    first,
    selected,
  ]);
  assert.deepEqual(sourceChoices([first]), [first]);
});

const { parseStorageSettingsInfo, parseStorageDirectoryChoice } =
  await import("@game4/ipc-contracts");
test("storage settings decode private path observations and reject malformed responses", () => {
  const settings = examples.storageSettings;
  assert.deepEqual(parseStorageSettingsInfo(settings), settings);
  assert.deepEqual(parseStorageDirectoryChoice({ cancelled: true, settings }), {
    cancelled: true,
    settings,
  });
  for (const downloads of [
    undefined,
    { path: "", available: true },
    { path: "bad\u0000path", available: true },
    { path: "C:\\Games", available: "yes" },
  ])
    assert.throws(
      () => parseStorageSettingsInfo({ ...settings, downloads }),
      code("INVALID_RESPONSE"),
    );
  assert.throws(
    () => parseStorageDirectoryChoice({ cancelled: "true", settings }),
    code("INVALID_RESPONSE"),
  );
});
test("storage operations are capability gated and accept directory kinds rather than writable paths", async () => {
  const calls = [];
  const client = make(async (command, args) => {
    calls.push(command);
    if (command === "app_info") return examples.appInfo;
    if (command === "storage_settings_get") return examples.storageSettings;
    assert.deepEqual(args, { request: { kind: "downloads" } });
    return command === "storage_directory_choose"
      ? { cancelled: true, settings: examples.storageSettings }
      : { ...examples.storageSettings, downloads: null };
  });
  await client.getStorageSettings();
  assert.equal(
    (await client.chooseStorageDirectory({ kind: "downloads" })).cancelled,
    true,
  );
  assert.equal(
    (await client.resetStorageDirectory({ kind: "downloads" })).downloads,
    null,
  );
  const count = calls.length;
  await assert.rejects(
    client.chooseStorageDirectory({ kind: "arbitrary" }),
    code("INVALID_ARGUMENT"),
  );
  await assert.rejects(
    client.chooseStorageDirectory({ kind: "downloads", path: "C:\\Elsewhere" }),
    code("INVALID_ARGUMENT"),
  );
  assert.equal(calls.length, count);
  const old = make(async (command) => {
    assert.equal(command, "app_info");
    return {
      ...examples.appInfo,
      capabilities: examples.appInfo.capabilities.filter(
        (c) => !c.startsWith("storage_"),
      ),
    };
  });
  await assert.rejects(
    old.getStorageSettings(),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.chooseStorageDirectory({ kind: "downloads" }),
    code("CAPABILITY_UNAVAILABLE"),
  );
});

const { parseAssociatedFile, parseAssociatedFileResult } =
  await import("@game4/ipc-contracts");
const associated = {
  token: "7b22b2eb-a47e-43e2-975b-8c01b541db17",
  fileName: "Coleção ∞.g4",
  kind: "g4",
};
const associatedPackage = {
  fileName: associated.fileName,
  packageVersion: 1,
  collectionId: associated.token,
  title: "Fixture",
  fileCount: 3,
  totalBytes: "40",
  verified: true,
};
test("associated file responses preserve native tokens and enforce format-specific inspection results", () => {
  assert.equal(parseAssociatedFile(null), null);
  assert.deepEqual(parseAssociatedFile(associated), associated);
  assert.deepEqual(
    parseAssociatedFileResult({
      kind: "g4",
      package: associatedPackage,
      manifest: null,
    }).package,
    associatedPackage,
  );
  assert.equal(
    parseAssociatedFileResult({
      kind: "g4e",
      package: null,
      manifest: examples.everOpen.manifest,
    }).manifest.contentVerified,
    false,
  );
  for (const value of [
    { ...associated, token: "path" },
    { ...associated, kind: "exe" },
    { ...associated, fileName: "C:\\file.g4" },
  ])
    assert.throws(() => parseAssociatedFile(value), code("INVALID_RESPONSE"));
  for (const value of [
    { kind: "g4", package: null, manifest: null },
    {
      kind: "g4e",
      package: associatedPackage,
      manifest: examples.everOpen.manifest,
    },
    {
      kind: "g4",
      package: { ...associatedPackage, verified: false },
      manifest: null,
    },
  ])
    assert.throws(
      () => parseAssociatedFileResult(value),
      code("INVALID_RESPONSE"),
    );
});
test("associated file inspection is capability gated and forwards only an opaque native token", async () => {
  const client = make(async (command, args) => {
    if (command === "app_info") return examples.appInfo;
    if (command === "associated_file_next") return associated;
    assert.equal(command, "associated_file_read");
    assert.deepEqual(args, { request: { token: associated.token } });
    return { kind: "g4", package: associatedPackage, manifest: null };
  });
  assert.equal(
    (await client.nextAssociatedFile()).fileName,
    associated.fileName,
  );
  assert.equal(
    (await client.readAssociatedFile({ token: associated.token })).package
      .verified,
    true,
  );
  const old = make(async (command) => {
    assert.equal(command, "app_info");
    return {
      ...examples.appInfo,
      capabilities: examples.appInfo.capabilities.filter(
        (c) => !c.startsWith("associated_file_"),
      ),
    };
  });
  await assert.rejects(
    old.nextAssociatedFile(),
    code("CAPABILITY_UNAVAILABLE"),
  );
  await assert.rejects(
    old.readAssociatedFile({ token: associated.token }),
    code("CAPABILITY_UNAVAILABLE"),
  );
});

const { addNotice, observeTasks, observeTransfers } =
  await import("../src/services/notifications.ts");
test("notification history is bounded, deduplicates event identities and keeps explicit actions", () => {
  let items = [];
  for (let id = 1; id <= 40; id++)
    items = addNotice(items, { key: "notifications.transferPaused" }, id);
  assert.equal(items.length, 30);
  assert.equal(items[0].id, 11);
  items = addNotice(
    items,
    { key: "notifications.importComplete", dedupeKey: "job:1" },
    41,
  );
  assert.equal(
    addNotice(
      items,
      { key: "notifications.importComplete", dedupeKey: "job:1" },
      42,
    ),
    items,
  );
  assert.equal(items.at(-1).unread, true);
});
test("activity notifications ignore old history and progress ticks but report watched terminal transitions once", () => {
  const previous = new Map();
  const old = { id: "old", kind: "import", status: "succeeded" };
  assert.deepEqual(observeTasks(previous, [old]), []);
  const running = { id: "new", kind: "verify", status: "running" };
  assert.deepEqual(observeTasks(previous, [old, running]), []);
  assert.deepEqual(
    observeTasks(previous, [old, { ...running, processedCount: "9" }]),
    [],
  );
  const updates = observeTasks(previous, [
    old,
    { ...running, status: "succeeded" },
  ]);
  assert.equal(updates.length, 1);
  assert.equal(updates[0].key, "notifications.filesChecked");
  assert.equal(
    observeTasks(previous, [old, { ...running, status: "succeeded" }]).length,
    0,
  );
  previous.set("failed", { kind: "import", status: "queued" });
  assert.equal(
    observeTasks(previous, [
      { id: "failed", kind: "import", status: "failed" },
    ])[0].kind,
    "error",
  );
});
test("transfer notices distinguish completed downloads, continued sharing, errors and historical rows", () => {
  const previous = new Map();
  const row = {
    id: "a",
    name: "Fixture",
    mode: "download",
    status: "completed",
    isSeeding: false,
  };
  assert.deepEqual(observeTransfers(previous, [row]), []);
  previous.set("a", { status: "active", isSeeding: false });
  assert.equal(
    observeTransfers(previous, [row])[0].key,
    "notifications.downloadComplete",
  );
  assert.deepEqual(observeTransfers(previous, [row]), []);
  previous.set("a", { status: "active", isSeeding: false });
  const seed = { ...row, status: "active", isSeeding: true };
  assert.equal(
    observeTransfers(previous, [seed])[0].key,
    "notifications.downloadComplete",
  );
  assert.deepEqual(observeTransfers(previous, [row]), []);
  previous.set("a", { status: "active", isSeeding: false });
  assert.deepEqual(observeTransfers(previous, [{ ...seed, mode: "seed" }]), []);
  assert.equal(
    observeTransfers(previous, [
      { ...seed, status: "failed", isSeeding: false },
    ])[0].key,
    "notifications.transferFailed",
  );
  assert.deepEqual(observeTransfers(previous, []), []);
  assert.equal(previous.size, 0);
});

import { parseArchiveResponse } from "@game4/ipc-contracts";
test("archive responses validate discriminants, shape, raster bytes and pagination bounds", () => {
  assert.deepEqual(
    parseArchiveResponse({ kind: "done", id: null, cancelled: true }),
    { kind: "done", id: null, cancelled: true },
  );
  assert.deepEqual(
    parseArchiveResponse({ kind: "games", items: [], total: 0, offset: 0 })
      .items,
    [],
  );
  for (const value of [
    null,
    { kind: "executeSql" },
    { kind: "games", items: [], total: -1, offset: 0 },
    { kind: "done", id: 1, cancelled: false },
    { kind: "asset", bytes: [256], mime: "image/png" },
    { kind: "asset", bytes: [0], mime: "image/svg+xml" },
  ])
    assert.throws(() => parseArchiveResponse(value), code("INVALID_RESPONSE"));
});
test("archive artwork CSP permits only local blobs, not arbitrary network image sources", async () => {
  const config = JSON.parse(
    await readFile(
      new URL("../src-tauri/tauri.conf.json", import.meta.url),
      "utf8",
    ),
  );
  for (const csp of [config.app.security.csp, config.app.security.devCsp]) {
    const directive = csp
      .split(";")
      .find((s) => s.trim().startsWith("img-src"));
    assert.match(directive, /blob:/);
    assert.doesNotMatch(directive, /https:|\*/);
  }
});

import { matchingArchiveTransfers } from "../src/services/archive-metrics.ts";
test("archive telemetry matches only explicit info hashes and does not imply network availability", () => {
  const hash = "0123456789012345678901234567890123456789";
  const transfer = { id: "matching", infoHash: hash };
  assert.deepEqual(
    matchingArchiveTransfers(
      [{ magnet: `magnet:?xt=urn:btih:${hash}` }],
      [transfer, { id: "other", infoHash: "a".repeat(40) }],
    ),
    [transfer],
  );
  for (const magnet of [
    "https://example.test/",
    `magnet:?xt=urn:btih:${hash}&xt=urn:btih:${"a".repeat(40)}`,
    "not a magnet",
  ])
    assert.deepEqual(matchingArchiveTransfers([{ magnet }], [transfer]), []);
});

test("archive static action messages resolve to user-facing translations", async () => {
  const english = JSON.parse(
    await readFile(new URL("../src/locales/en.json", import.meta.url), "utf8"),
  );
  const files = [
    "../src/pages/ArchivePage.tsx",
    "../src/components/archive/ArchiveCards.tsx",
    "../src/components/archive/ArchiveEditor.tsx",
    "../src/components/archive/ArchiveLiveTransfers.tsx",
  ];
  for (const file of files) {
    const code = await readFile(new URL(file, import.meta.url), "utf8");
    for (const match of code.matchAll(
      /"((?:archive|notifications)\.[a-zA-Z]+(?:\.[a-zA-Z]+)*)"/g,
    )) {
      const translatedKey =
        match[1].split(".").reduce((obj, key) => obj?.[key], english) ===
        undefined
          ? match[1] + "_other"
          : match[1];
      const value = translatedKey
        .split(".")
        .reduce((obj, key) => obj?.[key], english);
      assert.equal(
        typeof value,
        "string",
        `Missing user-facing string: ${match[1]}`,
      );
    }
  }
});

import { parseEverOpenResult } from "@game4/ipc-contracts";
test("G4EP summary inspection keeps fragment references out of original-download controls", async () => {
  const doc = JSON.parse(
    await readFile(
      new URL(
        "../../../packages/g4e-format/examples/g4ep-fragments-v2-signed.g4e",
        import.meta.url,
      ),
      "utf8",
    ),
  );
  const summary = {
    fileName: "synthetic.g4e",
    schemaVersion: 2,
    representation: "g4ep_fragments",
    title: doc.manifest.collection.title,
    fileCount: 1,
    totalBytes: doc.manifest.files[0].sizeBytes,
    magnets: [],
    contentVerified: false,
    g4ep: {
      contentId: doc.contentId,
      protocolVersion: 1,
      codecId: doc.g4ep.coding.codecId,
      groupCount: 1,
      fragmentCount: 12,
      payloadSha256: doc.authentication.payloadSha256,
      signerKeyIds: doc.authentication.signatures.map((s) => s.keyId),
      publisherTrusted: false,
    },
  };
  const response = { cancelled: false, manifest: summary };
  assert.deepEqual(parseEverOpenResult(response), response);
  for (const mutate of [
    (s) => s.magnets.push(doc.discovery[1].magnet),
    (s) => (s.g4ep.publisherTrusted = true),
    (s) => (s.g4ep.fragmentCount = 11),
    (s) => (s.schemaVersion = 1),
    (s) => (s.contentVerified = true),
    (s) => s.g4ep.signerKeyIds.push(s.g4ep.signerKeyIds[0]),
  ]) {
    const bad = structuredClone(summary);
    mutate(bad);
    assert.throws(
      () => parseEverOpenResult({ cancelled: false, manifest: bad }),
      code("INVALID_RESPONSE"),
    );
  }
});

test("Guardian snapshots validate capacity, scoped permissions, inventory bounds and offline state", async () => {
  const { parseGuardianSnapshot } = await import("@game4/ipc-contracts");
  const value = {
    nodeId: "a".repeat(64),
    directory: "C:\\Guardian",
    config: {
      active: false,
      quotaBytes: "1048576",
      uploadBps: 1024,
      downloadBps: 2048,
    },
    usedBytes: "65536",
    fragmentCount: 1,
    health: "ready",
    entries: [
      {
        id: 1,
        contentId: "urn:sha256:" + "b".repeat(64),
        name: "test.bin",
        group: "0",
        index: 0,
        sha256: "c".repeat(64),
        sizeBytes: 65536,
        sharing: false,
        health: "verified",
      },
    ],
    offset: 0,
    total: 1,
    networkStarted: false,
  };
  assert.deepEqual(parseGuardianSnapshot(value), value);
  for (const modify of [
    (d) => (d.config.quotaBytes = "01"),
    (d) => (d.config.uploadBps = -1),
    (d) => (d.config.downloadBps = 1e10),
    (d) => (d.entries[0].sharing = "yes"),
    (d) => (d.entries[0].index = 12),
    (d) => (d.entries[0].name = "../test.bin"),
    (d) => (d.entries[0].sizeBytes = 0),
    (d) => (d.entries[0].sha256 = "bad"),
    (d) => d.entries.push(d.entries[0]),
    (d) => (d.total = 0),
    (d) => (d.unknown = true),
  ]) {
    const bad = structuredClone(value);
    modify(bad);
    assert.throws(() => parseGuardianSnapshot(bad), code("INVALID_RESPONSE"));
  }
});

test("Guardian network snapshots enforce explicit peers, TLS metadata and bounded outcomes", async () => {
  const { parseNetworkSnapshot } = await import("@game4/ipc-contracts");
  const value = {
    running: false,
    nodeId: "a".repeat(64),
    certificateSha256: "b".repeat(64),
    config: {
      bind: "127.0.0.1:42430",
      maxConnections: 4,
      timeoutSeconds: 120,
      retries: 2,
      peers: [
        {
          version: 1,
          nodeId: "c".repeat(64),
          address: "127.0.0.1:42431",
          certificateSha256: "d".repeat(64),
          certificateDer: "abcd",
        },
      ],
    },
    connections: 0,
    outcome: null,
  };
  assert.deepEqual(parseNetworkSnapshot(value), value);
  for (const modify of [
    (d) => (d.connections = 1),
    (d) => (d.config.maxConnections = 99),
    (d) => (d.config.timeoutSeconds = 0),
    (d) => (d.config.retries = 4),
    (d) => d.config.peers.push(d.config.peers[0]),
    (d) => (d.config.peers[0].nodeId = d.nodeId),
    (d) => (d.config.peers[0].certificateDer = "private"),
    (d) => (d.privateKey = "secret"),
    (d) =>
      (d.outcome = {
        operation: "download",
        peerId: "e".repeat(64),
        bytes: "1",
        attempts: 1,
        available: null,
      }),
  ]) {
    const bad = structuredClone(value);
    modify(bad);
    assert.throws(() => parseNetworkSnapshot(bad), code("INVALID_RESPONSE"));
  }
  const complete = structuredClone(value);
  complete.outcome = {
    operation: "download",
    peerId: "c".repeat(64),
    bytes: "1048576",
    attempts: 2,
    available: null,
  };
  assert.deepEqual(parseNetworkSnapshot(complete), complete);
});
