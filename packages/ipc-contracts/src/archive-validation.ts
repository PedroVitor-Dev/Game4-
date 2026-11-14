import type { ArchiveResponse } from "./generated.ts";
import { failure } from "./validation.ts";
type Shape =
  | "s"
  | "n"
  | "b"
  | { nullable: Shape }
  | { array: Shape }
  | { [key: string]: Shape };
const nullable = (shape: Shape): Shape => ({ nullable: shape });
const array = (shape: Shape): Shape => ({ array: shape });
const platform = {
  id: "s",
  name: "s",
  manufacturer: "s",
  year: nullable("n"),
  description: "s",
} satisfies Shape;
const game = {
  id: "s",
  title: "s",
  year: nullable("n"),
  developer: "s",
  publisher: "s",
  genre: "s",
  description: "s",
  history: "s",
  createdAt: "s",
  updatedAt: "s",
} satisfies Shape;
const card = {
  game,
  platforms: "s",
  region: "s",
  coverId: nullable("s"),
  favorite: "b",
  inLibrary: "b",
  localFiles: "n",
  sourceCount: "n",
  updates: "n",
  dlcs: "n",
} satisfies Shape;
const platformCard = {
  platform,
  coverId: nullable("s"),
  games: "n",
  withFiles: "n",
} satisfies Shape;
const catalog = {
  id: "s",
  name: "s",
  digest: "s",
  attribution: "s",
  importedAt: "s",
} satisfies Shape;
const collection = { id: "s", name: "s" } satisfies Shape;
const responseShapes: Record<string, Shape> = {
  overview: {
    kind: "s",
    stats: {
      games: "n",
      platforms: "n",
      localFiles: "n",
      sources: "n",
      catalogs: "n",
    },
    platforms: array(platformCard),
    recent: array(card),
    updated: array(card),
    collections: array(collection),
    catalogs: array(catalog),
  },
  platforms: { kind: "s", platforms: array(platformCard) },
  games: { kind: "s", items: array(card), total: "n", offset: "n" },
  game: {
    kind: "s",
    card,
    releases: array({
      id: "s",
      gameId: "s",
      platformId: "s",
      region: "s",
      languages: "s",
      titleId: "s",
    }),
    editions: array({ id: "s", releaseId: "s", name: "s", medium: "s" }),
    contents: array({
      id: "s",
      editionId: "s",
      kind: "s",
      name: "s",
      version: "s",
      build: "s",
      releaseDate: "s",
      format: "s",
      baseId: nullable("s"),
      description: "s",
      changelog: "s",
      identifier: "s",
    }),
    dependencies: array({ contentId: "s", requiredId: "s" }),
    files: array({
      id: "s",
      contentId: "s",
      name: "s",
      format: "s",
      sizeBytes: "s",
      sha256: "s",
      itemId: nullable("s"),
      fileId: nullable("s"),
    }),
    sources: array({
      id: "s",
      contentId: "s",
      magnet: "s",
      license: "s",
      attribution: "s",
      authorized: "b",
    }),
    assets: array({
      id: "s",
      platformId: nullable("s"),
      gameId: nullable("s"),
      contentId: nullable("s"),
      role: "s",
      source: "s",
      license: "s",
      attribution: "s",
    }),
    states: array({
      fileId: "s",
      status: "s",
      checkedAt: nullable("s"),
      location: nullable("s"),
    }),
    provenance: array({
      id: "s",
      gameId: "s",
      source: "s",
      attribution: "s",
      note: "s",
      createdAt: "s",
    }),
    catalogs: array(catalog),
    collections: array("s"),
  },
  asset: { kind: "s", bytes: array("n"), mime: "s" },
  catalogPreview: {
    kind: "s",
    token: "s",
    name: "s",
    games: "n",
    platforms: "n",
    attribution: "s",
  },
  done: { kind: "s", id: nullable("s"), cancelled: "b" },
};
function matches(value: unknown, shape: Shape, depth = 0): boolean {
  if (depth > 12) return false;
  if (shape === "s") return typeof value === "string" && value.length <= 65536;
  if (shape === "n")
    return (
      typeof value === "number" && Number.isSafeInteger(value) && value >= 0
    );
  if (shape === "b") return typeof value === "boolean";
  if ("nullable" in shape)
    return value === null || matches(value, shape.nullable, depth + 1);
  if ("array" in shape)
    return (
      Array.isArray(value) &&
      value.length <= 4 * 1024 * 1024 &&
      value.every((v) => matches(v, shape.array, depth + 1))
    );
  return (
    typeof value === "object" &&
    value !== null &&
    !Array.isArray(value) &&
    Object.entries(shape).every(
      ([key, s]) =>
        Object.prototype.hasOwnProperty.call(value, key) &&
        matches((value as Record<string, unknown>)[key], s, depth + 1),
    )
  );
}
export function parseArchiveResponse(value: unknown): ArchiveResponse {
  const kind =
    typeof value === "object" && value !== null
      ? (value as { kind?: unknown }).kind
      : undefined;
  if (
    typeof kind !== "string" ||
    !Object.prototype.hasOwnProperty.call(responseShapes, kind) ||
    !matches(value, responseShapes[kind])
  )
    throw failure("INVALID_RESPONSE");
  const parsed = value as ArchiveResponse;
  if (
    parsed.kind === "asset" &&
    (!["image/png", "image/jpeg", "image/webp"].includes(parsed.mime) ||
      parsed.bytes.some((n) => n > 255))
  )
    throw failure("INVALID_RESPONSE");
  if (parsed.kind === "games" && parsed.items.length > 48)
    throw failure("INVALID_RESPONSE");
  return parsed;
}
