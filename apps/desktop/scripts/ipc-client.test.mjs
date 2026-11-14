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
