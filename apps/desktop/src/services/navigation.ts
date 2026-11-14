import type { LibraryItemSummary } from "@game4/ipc-contracts";
export type Page =
  | "dashboard"
  | "library"
  | "download"
  | "share"
  | "transfers"
  | "community"
  | "tasks"
  | "settings";
export type Navigate = (
  page: Page,
  source?: LibraryItemSummary,
  magnet?: string,
) => void;
export function transferView(
  page: Page,
): "download" | "seed" | "sessions" | null {
  return page === "download"
    ? "download"
    : page === "share"
      ? "seed"
      : page === "transfers"
        ? "sessions"
        : null;
}
export function sourceChoices(
  items: LibraryItemSummary[],
  source?: LibraryItemSummary,
) {
  return source && !items.some((item) => item.id === source.id)
    ? [source, ...items]
    : items;
}
