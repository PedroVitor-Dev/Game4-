// Read only the explicitly selected descriptor; never dereference its hints.
import { open } from "node:fs/promises";
import { MAX_BYTES, parseG4e, G4eValidationError } from "./validation.mjs";
try {
  if (process.argv.length !== 3)
    throw new Error("Usage: npm.cmd run g4e:validate -- PATH.g4e");
  const file = await open(process.argv[2], "r");
  const chunks = [];
  let total = 0;
  try {
    if (!(await file.stat()).isFile())
      throw new Error("Descriptor must be a regular file");
    while (total <= MAX_BYTES) {
      const buffer = Buffer.allocUnsafe(
        Math.min(1024 * 1024, MAX_BYTES + 1 - total),
      );
      const { bytesRead } = await file.read(buffer, 0, buffer.length, null);
      if (bytesRead === 0) break;
      chunks.push(buffer.subarray(0, bytesRead));
      total += bytesRead;
    }
  } finally {
    await file.close();
  }
  const doc = parseG4e(Buffer.concat(chunks, total));
  console.log(
    JSON.stringify({
      valid: true,
      schemaVersion: doc.schemaVersion,
      representation: doc.representation,
      fileCount: doc.manifest.files.length,
      torrentCount: doc.torrents.length,
      networkAccess: false,
    }),
  );
} catch (error) {
  console.error(
    JSON.stringify({
      valid: false,
      code:
        error instanceof G4eValidationError ? error.code : "G4E_READ_FAILED",
      message:
        error instanceof G4eValidationError
          ? error.message
          : "Unable to read descriptor; pass one readable .g4e path",
    }),
  );
  process.exitCode = 1;
}
