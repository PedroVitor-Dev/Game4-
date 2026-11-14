import { useCallback, useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type {
  ArchiveSearch,
  ArchiveRequest,
  ArchivePlatform,
  ArchiveGame,
  ArchiveGameCard,
  ArchiveContent,
  ArchiveSource,
  LibraryItemSummary,
  ArchivePlatformCard,
  ArchiveResponse,
} from "@game4/ipc-contracts";
import {
  archiveRequest,
  invalidateArchive,
  preference,
  savePreference,
  preservationState,
  type ArchiveOverview,
  type GameDetail,
} from "../services/archive";
import {
  isTauri,
  listLibrary,
  verifyItem,
  exportEver,
} from "../services/desktop";
import { formatBytes } from "../services/file-identification";
import type { Navigate } from "../services/navigation";
import { useNotifications } from "../components/Notifications";
import { EmptyState, ErrorNotice } from "../components/Feedback";
import {
  ArchiveEditor,
  type EditorSpec,
  type FormValues,
  type FormField,
} from "../components/archive/ArchiveEditor";
import {
  PlatformCard,
  GameCoverCard,
  GameHero,
  ArtworkGallery,
  VersionCard,
  UpdateCard,
  DlcCard,
  PreservationBadge,
  DownloadPanel,
  ArchiveSkeleton,
} from "../components/archive/ArchiveCards";
import "./archive.css";
import { ArchiveLiveTransfers } from "../components/archive/ArchiveLiveTransfers";
type View =
  "archive" | "platforms" | "collections" | "preservation" | "community";
const initialQuery: ArchiveSearch = {
  search: "",
  platformId: "",
  region: "",
  genre: "",
  language: "",
  edition: "",
  year: null,
  availability: "",
  order: "name",
  offset: 0,
  scope: "",
  collectionId: "",
  catalogId: "",
};
const uid = () => crypto.randomUUID();
function strings(value: object): FormValues {
  return Object.fromEntries(
    Object.entries(value).map(([k, v]) => [
      k,
      typeof v === "boolean"
        ? v
        : v === null || v === undefined
          ? ""
          : String(v),
    ]),
  );
}
const field = (
  name: string,
  required = false,
  type?: FormField["type"],
): FormField => ({ name, required, type });
export function ArchivePage({
  view,
  onNavigate,
}: {
  view: View;
  onNavigate: Navigate;
}) {
  const { t, i18n } = useTranslation();
  const { notify, watchTask } = useNotifications();
  const [overview, setOverview] = useState<ArchiveOverview>();
  const [items, setItems] = useState<ArchiveGameCard[]>([]);
  const [total, setTotal] = useState(0);
  const [query, setQuery] = useState<ArchiveSearch>(() => ({
    ...initialQuery,
    order: preference("order", "name", ["name", "year", "recent", "updated"]),
  }));
  const [layout, setLayout] = useState(() =>
    preference("layout", "grid", ["grid", "compact", "list"]),
  );
  const [gameId, setGameId] = useState<string>();
  const [detail, setDetail] = useState<GameDetail>();
  const [tab, setTab] = useState("overview");
  const [contentLimit, setContentLimit] = useState(24);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>();
  const [editor, setEditor] = useState<EditorSpec>();
  const [revision, setRevision] = useState(0);
  const [preview, setPreview] =
    useState<Extract<ArchiveResponse, { kind: "catalogPreview" }>>();
  const [localItems, setLocalItems] = useState<LibraryItemSummary[]>([]);
  const [localCursor, setLocalCursor] = useState<string | null>(null);
  const [localSearch, setLocalSearch] = useState("");
  const [linkOpen, setLinkOpen] = useState(false);
  const [localLoading, setLocalLoading] = useState(false);
  const [localError, setLocalError] = useState<unknown>();
  const [linkContent, setLinkContent] = useState("");
  const [localLoaded, setLocalLoaded] = useState(false);
  const [downloadSource, setDownloadSource] = useState<ArchiveSource>();
  const [rights, setRights] = useState(false);
  const [liveDetail, setLiveDetail] = useState(false);
  const alive = useRef(true);
  useEffect(() => {
    alive.current = true;
    return () => {
      alive.current = false;
    };
  }, []);
  const refresh = useCallback(() => {
    invalidateArchive();
    setRevision((n) => n + 1);
  }, []);
  useEffect(() => {
    let active = true;
    if (!isTauri()) {
      setLoading(false);
      return;
    }
    archiveRequest({ kind: "overview" }, true)
      .then((r) => {
        if (active && r.kind === "overview") setOverview(r);
      })
      .catch((e) => {
        if (active) setError(e);
      });
    return () => {
      active = false;
    };
  }, [revision]);
  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    setLoading(true);
    setError(undefined);
    const timer = setTimeout(() => {
      const request: ArchiveRequest = gameId
        ? { kind: "game", id: gameId }
        : { kind: "games", query };
      archiveRequest(request, true)
        .then((r) => {
          if (!active) return;
          if (r.kind === "game") setDetail(r);
          else if (r.kind === "games") {
            setItems(r.items);
            setTotal(r.total);
          }
        })
        .catch((e) => {
          if (active) setError(e);
        })
        .finally(() => {
          if (active) setLoading(false);
        });
    }, 180);
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [gameId, query, revision]);
  useEffect(() => {
    if (!gameId || !liveDetail) return;
    let active = true;
    const timer = setInterval(() => {
      archiveRequest({ kind: "game", id: gameId })
        .then((r) => {
          if (active && r.kind === "game") setDetail(r);
        })
        .catch(() => {});
    }, 3000);
    return () => {
      active = false;
      clearInterval(timer);
    };
  }, [gameId, liveDetail]);
  const platforms = overview?.platforms ?? [];
  const platform = platforms.find((p) => p.platform.id === query.platformId);
  function filters(update: Partial<ArchiveSearch>) {
    setQuery((q) => ({ ...q, ...update, offset: 0 }));
  }
  async function action(fn: () => Promise<unknown>, message = "archive.saved") {
    setBusy(true);
    setError(undefined);
    try {
      const result = await fn();
      if (
        result &&
        typeof result === "object" &&
        "cancelled" in result &&
        result.cancelled
      )
        return;
      notify({ key: message, kind: "success" });
      refresh();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  async function save(request: ArchiveRequest) {
    const result = await archiveRequest(request);
    if (result.kind !== "done" || !result.cancelled) {
      notify({ key: "archive.saved", kind: "success" });
      refresh();
    }
    return result;
  }
  function form(
    title: string,
    initial: FormValues,
    fields: EditorSpec["fields"],
    build: (v: FormValues) => ArchiveRequest,
  ) {
    setEditor({
      title,
      initial,
      fields,
      submit: async (v) => {
        const request = build(v);
        const result = await save(request);
        if (request.kind === "saveGame" && result.kind === "done" && result.id)
          openGame(result.id);
      },
    });
  }
  function platformForm(value?: ArchivePlatform) {
    const v = value ?? {
      id: uid(),
      name: "",
      manufacturer: "",
      year: null,
      description: "",
    };
    form(
      value ? "archive.editPlatform" : "archive.addPlatform",
      strings(v),
      [
        field("name", true),
        field("manufacturer", true),
        field("year", false, "number"),
        field("description", false, "textarea"),
      ],
      (f) => ({
        kind: "savePlatform",
        value: {
          ...v,
          name: String(f.name),
          manufacturer: String(f.manufacturer),
          year: f.year ? Number(f.year) : null,
          description: String(f.description),
        },
      }),
    );
  }
  function gameForm(value?: ArchiveGame) {
    const v = value ?? {
      id: uid(),
      title: "",
      year: null,
      developer: "",
      publisher: "",
      genre: "",
      description: "",
      history: "",
      createdAt: "",
      updatedAt: "",
    };
    form(
      value ? "archive.editGame" : "archive.addGame",
      strings(v),
      [
        field("title", true),
        field("year", false, "number"),
        field("developer"),
        field("publisher"),
        field("genre"),
        field("description", false, "textarea"),
        field("history", false, "textarea"),
      ],
      (f) => ({
        kind: "saveGame",
        value: {
          ...v,
          title: String(f.title),
          year: f.year ? Number(f.year) : null,
          developer: String(f.developer),
          publisher: String(f.publisher),
          genre: String(f.genre),
          description: String(f.description),
          history: String(f.history),
        },
      }),
    );
  }
  function releaseForm(existing?: GameDetail["releases"][number]) {
    if (!detail) return;
    const v = existing ?? {
      id: uid(),
      gameId: detail.card.game.id,
      platformId: query.platformId,
      region: "",
      languages: "",
      titleId: "",
    };
    form(
      "archive.addRelease",
      strings(v),
      [
        {
          name: "platformId",
          required: true,
          disabled: !!existing,
          options: platforms.map((p) => ({
            value: p.platform.id,
            label: p.platform.name,
          })),
        },
        field("region"),
        field("languages"),
        field("titleId"),
      ],
      (f) => ({
        kind: "saveRelease",
        value: {
          ...v,
          platformId: String(f.platformId),
          region: String(f.region),
          languages: String(f.languages),
          titleId: String(f.titleId),
        },
      }),
    );
  }
  function editionForm(existing?: GameDetail["editions"][number]) {
    if (!detail) return;
    const v = existing ?? { id: uid(), releaseId: "", name: "", medium: "" };
    form(
      "archive.addEdition",
      strings(v),
      [
        {
          name: "releaseId",
          required: true,
          disabled: !!existing,
          options: detail.releases.map((r) => ({
            value: r.id,
            label: `${platforms.find((p) => p.platform.id === r.platformId)?.platform.name ?? r.platformId} · ${r.region || t("archive.unspecified")}`,
          })),
        },
        field("name", true),
        field("medium"),
      ],
      (f) => ({
        kind: "saveEdition",
        value: {
          ...v,
          releaseId: String(f.releaseId),
          name: String(f.name),
          medium: String(f.medium),
        },
      }),
    );
  }
  function contentForm(kind: string, existing?: ArchiveContent) {
    if (!detail) return;
    const v = existing ?? {
      id: uid(),
      editionId: "",
      kind,
      name: "",
      version: "",
      build: "",
      releaseDate: "",
      format: "",
      baseId: null,
      description: "",
      changelog: "",
      identifier: "",
    };
    form(
      `archive.add${kind[0].toUpperCase() + kind.slice(1)}`,
      strings(v),
      (f) => [
        {
          name: "editionId",
          required: true,
          disabled: !!existing,
          options: detail.editions.map((e) => ({
            value: e.id,
            label: editionLabel(e.id),
          })),
        },
        field("name", true),
        field("version"),
        ...(kind !== "version"
          ? [
              {
                name: "baseId",
                required: true,
                options: detail.contents
                  .filter(
                    (c) => c.kind === "version" && c.editionId === f.editionId,
                  )
                  .map((c) => ({
                    value: c.id,
                    label: `${c.name} · ${c.version}`,
                  })),
              },
            ]
          : []),
        field("build"),
        field("releaseDate", false, "date"),
        field("format"),
        field("identifier"),
        field("description", false, "textarea"),
        field("changelog", false, "textarea"),
      ],
      (f) => ({
        kind: "saveContent",
        value: {
          ...v,
          editionId: String(f.editionId),
          name: String(f.name),
          version: String(f.version),
          build: String(f.build),
          releaseDate: String(f.releaseDate),
          format: String(f.format),
          identifier: String(f.identifier),
          baseId: kind === "version" ? null : String(f.baseId),
          description: String(f.description),
          changelog: String(f.changelog),
        },
      }),
    );
  }
  function editionLabel(id: string) {
    const edition = detail?.editions.find((e) => e.id === id);
    const release = detail?.releases.find((r) => r.id === edition?.releaseId);
    const p = platforms.find((p) => p.platform.id === release?.platformId);
    return [
      p?.platform.name,
      edition?.name,
      release?.region,
      release?.languages,
    ]
      .filter(Boolean)
      .join(" · ");
  }
  function assetForm(ownerKind: string, ownerId: string) {
    form(
      "archive.addArtwork",
      {
        ownerKind,
        ownerId,
        role: ownerKind === "platform" ? "console" : "cover",
        source: "",
        license: "",
        attribution: "",
      },
      [
        {
          name: "role",
          required: true,
          options: (ownerKind === "platform"
            ? ["console", "banner"]
            : ["cover", "banner", "screenshot"]
          ).map((v) => ({ value: v, label: t(`archive.roles.${v}`) })),
        },
        field("source", true),
        field("license", true),
        field("attribution"),
      ],
      (f) => ({
        kind: "addAsset",
        ownerKind,
        ownerId,
        role: String(f.role),
        source: String(f.source),
        license: String(f.license),
        attribution: String(f.attribution),
      }),
    );
  }
  function sourceForm(existing?: ArchiveSource) {
    if (!detail) return;
    const v = existing ?? {
      id: uid(),
      contentId: "",
      magnet: "",
      license: "",
      attribution: "",
      authorized: false,
    };
    form(
      "archive.addSource",
      strings(v),
      [
        {
          name: "contentId",
          required: true,
          options: detail.contents.map((c) => ({
            value: c.id,
            label: `${c.name} · ${editionLabel(c.editionId)}`,
          })),
        },
        field("magnet", true),
        field("license", true),
        field("attribution"),
        field("authorized", false, "checkbox"),
      ],
      (f) => ({
        kind: "saveSource",
        value: {
          ...v,
          contentId: String(f.contentId),
          magnet: String(f.magnet),
          license: String(f.license),
          attribution: String(f.attribution),
          authorized: f.authorized === true,
        },
      }),
    );
  }
  function attachForm() {
    if (!detail) return;
    form(
      "archive.attachEver",
      { contentId: "", license: "", authorized: false },
      [
        {
          name: "contentId",
          required: true,
          options: detail.contents.map((c) => ({ value: c.id, label: c.name })),
        },
        field("license", true),
        field("authorized", false, "checkbox"),
      ],
      (f) => ({
        kind: "attachEver",
        contentId: String(f.contentId),
        license: String(f.license),
        authorized: f.authorized === true,
      }),
    );
  }
  function dependencyForm(content: ArchiveContent) {
    if (!detail) return;
    form(
      "archive.addDependency",
      { contentId: content.id, requiredId: "" },
      [
        {
          name: "requiredId",
          required: true,
          options: detail.contents
            .filter((c) => c.id !== content.id)
            .map((c) => ({ value: c.id, label: `${c.name} · ${c.version}` })),
        },
      ],
      (f) => ({
        kind: "saveDependency",
        value: { contentId: content.id, requiredId: String(f.requiredId) },
      }),
    );
  }
  function collectionForm() {
    form("archive.addCollection", { name: "" }, [field("name", true)], (f) => ({
      kind: "saveCollection",
      value: { id: uid(), name: String(f.name) },
    }));
  }
  function provenanceForm() {
    if (!detail) return;
    form(
      "archive.addAttribution",
      { source: "", attribution: "", note: "" },
      [
        field("source", true),
        field("attribution"),
        field("note", false, "textarea"),
      ],
      (f) => ({
        kind: "saveMetadataSource",
        value: {
          id: uid(),
          gameId: detail.card.game.id,
          source: String(f.source),
          attribution: String(f.attribution),
          note: String(f.note),
          createdAt: "",
        },
      }),
    );
  }
  async function favorite(card: ArchiveGameCard) {
    await action(() =>
      archiveRequest({
        kind: "preference",
        gameId: card.game.id,
        favorite: !card.favorite,
        inLibrary: card.inLibrary,
      }),
    );
  }
  function openGame(id: string) {
    setGameId(id);
    setDetail(undefined);
    setTab("overview");
    setDownloadSource(undefined);
    setRights(false);
    setContentLimit(24);
    setLiveDetail(false);
    window.scrollTo({ top: 0 });
    void archiveRequest({ kind: "visit", gameId: id }).catch(() => {});
  }
  async function readLocals(cursor: string | null = null) {
    setLocalLoading(true);
    setLocalError(undefined);
    try {
      const result = await listLibrary({
        query: localSearch || null,
        page: { cursor, limit: 50 },
      });
      if (alive.current) {
        setLocalItems(result.items);
        setLocalCursor(result.nextCursor);
        setLocalLoaded(true);
      }
    } catch (e) {
      setLocalError(e);
    } finally {
      setLocalLoading(false);
    }
  }
  useEffect(() => {
    if (!linkOpen) return;
    const timer = setTimeout(() => {
      void readLocals();
    }, 200);
    return () => clearTimeout(timer);
  }, [linkOpen, localSearch]);
  async function verify(itemId: string) {
    await action(async () => {
      const task = await verifyItem({ itemId, requestId: uid() });
      watchTask(task.taskId, "verify");
      setLiveDetail(true);
    }, "notifications.checkStarted");
  }
  async function prepareCatalog() {
    setBusy(true);
    setError(undefined);
    try {
      const r = await archiveRequest({ kind: "prepareCatalog" });
      if (r.kind === "catalogPreview") setPreview(r);
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  function exportForm() {
    form(
      "archive.exportCatalog",
      { name: "", attribution: "" },
      [field("name", true), field("attribution")],
      (f) => ({
        kind: "exportCatalog",
        name: String(f.name),
        attribution: String(f.attribution),
      }),
    );
  }
  const tabs = [
    "overview",
    "versions",
    "updates",
    "dlcs",
    "files",
    "preservation",
    "community",
  ];
  const grid = (cards: ArchiveGameCard[]) => (
    <div className={`archive-grid ${layout}`}>
      {cards.map((item) => (
        <GameCoverCard
          key={item.game.id}
          item={item}
          onOpen={() => openGame(item.game.id)}
          onFavorite={() => void favorite(item)}
        />
      ))}
    </div>
  );
  const platformGrid = (cards: ArchivePlatformCard[]) => (
    <div className="archive-platform-grid">
      {cards.map((item) => (
        <PlatformCard
          key={item.platform.id}
          item={item}
          onOpen={() => {
            filters({ platformId: item.platform.id, search: "" });
            window.scrollTo({ top: 0 });
          }}
          onEdit={() => platformForm(item.platform)}
        />
      ))}
    </div>
  );
  if (!isTauri())
    return (
      <EmptyState
        title="archive.previewTitle"
        description="archive.previewDescription"
      />
    );
  return (
    <div className="archive-workspace">
      <div className="archive-toolbar">
        <div className="archive-breadcrumb">
          <button
            onClick={() => {
              setGameId(undefined);
              setDetail(undefined);
              filters({ platformId: "", catalogId: "", collectionId: "" });
            }}
          >
            {t(`nav.${view}`)}
          </button>
          {platform && (
            <>
              <span>/</span>
              <button
                onClick={() => {
                  setGameId(undefined);
                  setDetail(undefined);
                }}
              >
                {platform.platform.name}
              </button>
            </>
          )}
          {detail && (
            <>
              <span>/</span>
              <span>{detail.card.game.title}</span>
            </>
          )}
        </div>
        <div className="actions">
          {view === "platforms" && (
            <button disabled={busy} onClick={() => platformForm()}>
              {t("archive.addPlatform")}
            </button>
          )}
          <button
            disabled={busy || platforms.length === 0}
            onClick={() => gameForm()}
          >
            {t("archive.addGame")}
          </button>
          <button disabled={busy} onClick={() => void prepareCatalog()}>
            {t("archive.importCatalog")}
          </button>
          <button disabled={busy || !overview} onClick={exportForm}>
            {t("archive.exportCatalog")}
          </button>
        </div>
      </div>
      {error !== undefined && (
        <ErrorNotice
          error={error}
          onRetry={refresh}
          onDismiss={() => setError(undefined)}
          busy={busy}
        />
      )}
      {preview && (
        <section className="content-card archive-catalog-preview">
          <h2>{t("archive.reviewCatalog")}</h2>
          <h3>{preview.name}</h3>
          <p>{preview.attribution}</p>
          <p>
            {t("archive.previewCounts", {
              games: preview.games,
              platforms: preview.platforms,
            })}
          </p>
          <p>{t("archive.importWarning")}</p>
          <div className="actions">
            <button disabled={busy} onClick={() => setPreview(undefined)}>
              {t("archive.cancel")}
            </button>
            <button
              className="primary"
              disabled={busy}
              onClick={() =>
                void action(async () => {
                  const r = await archiveRequest({
                    kind: "importCatalog",
                    token: preview.token,
                  });
                  setPreview(undefined);
                  return r;
                }, "archive.imported")
              }
            >
              {t("archive.confirmImport")}
            </button>
          </div>
        </section>
      )}
      {gameId ? (
        loading && !detail ? (
          <ArchiveSkeleton />
        ) : (
          detail && (
            <>
              <GameHero
                card={detail.card}
                banner={detail.assets.find((a) => a.role === "banner")?.id}
              >
                {detail.files.some((f) => f.itemId) ? (
                  <button
                    className="primary"
                    disabled={busy}
                    onClick={() => {
                      setTab("files");
                    }}
                  >
                    {t("archive.verifyFiles")}
                  </button>
                ) : detail.sources.length > 0 ? (
                  <button
                    className="primary"
                    onClick={() => {
                      setTab("files");
                      setDownloadSource(detail.sources[0]);
                      setRights(false);
                    }}
                  >
                    {t("archive.download")}
                  </button>
                ) : (
                  <button
                    className="primary"
                    disabled={busy || detail.card.inLibrary}
                    onClick={() =>
                      void action(() =>
                        archiveRequest({
                          kind: "preference",
                          gameId,
                          favorite: detail.card.favorite,
                          inLibrary: true,
                        }),
                      )
                    }
                  >
                    {t(
                      detail.card.inLibrary
                        ? "archive.inLibrary"
                        : "archive.addToLibrary",
                    )}
                  </button>
                )}
                <button
                  disabled={busy}
                  onClick={() => gameForm(detail.card.game)}
                >
                  {t("archive.editGame")}
                </button>
                <button
                  disabled={busy}
                  onClick={() => assetForm("game", gameId)}
                >
                  {t("archive.addArtwork")}
                </button>
                <button
                  aria-pressed={detail.card.favorite}
                  disabled={busy}
                  onClick={() => void favorite(detail.card)}
                >
                  {t(
                    detail.card.favorite
                      ? "archive.removeFavorite"
                      : "archive.addFavorite",
                  )}
                </button>
              </GameHero>
              <div
                className="archive-tabs"
                role="tablist"
                aria-label={t("archive.gameSections")}
              >
                {tabs.map((name, index) => (
                  <button
                    key={name}
                    role="tab"
                    id={`archive-tab-${name}`}
                    aria-selected={tab === name}
                    aria-controls="archive-tab-panel"
                    tabIndex={tab === name ? 0 : -1}
                    onClick={() => {
                      setTab(name);
                      setContentLimit(24);
                    }}
                    onKeyDown={(event) => {
                      const direction =
                        event.key === "ArrowRight"
                          ? 1
                          : event.key === "ArrowLeft"
                            ? -1
                            : 0;
                      if (direction) {
                        event.preventDefault();
                        const next =
                          tabs[(index + direction + tabs.length) % tabs.length];
                        setTab(next);
                        setContentLimit(24);
                        document.getElementById(`archive-tab-${next}`)?.focus();
                      }
                    }}
                  >
                    {t(`archive.tabs.${name}`)}
                  </button>
                ))}
              </div>
              <section
                id="archive-tab-panel"
                role="tabpanel"
                aria-labelledby={`archive-tab-${tab}`}
              >
                {tab === "overview" && (
                  <div className="archive-detail-columns">
                    <div>
                      <section className="content-card">
                        <h2>{t("archive.aboutGame")}</h2>
                        <p className="archive-prose">
                          {detail.card.game.description ||
                            t("archive.noDescription")}
                        </p>
                        {detail.card.game.history && (
                          <>
                            <h3>{t("archive.developmentHistory")}</h3>
                            <p className="archive-prose">
                              {detail.card.game.history}
                            </p>
                          </>
                        )}
                      </section>
                      <ArtworkGallery
                        assets={detail.assets.filter(
                          (a) => a.role === "screenshot",
                        )}
                        title={detail.card.game.title}
                      />
                      <section className="content-card">
                        <div className="archive-section-heading">
                          <h3>{t("archive.releases")}</h3>
                          <button onClick={() => releaseForm()}>
                            {t("archive.addRelease")}
                          </button>
                        </div>
                        {detail.releases.length === 0 && (
                          <p>{t("archive.noReleases")}</p>
                        )}
                        {detail.releases.map((r) => (
                          <div className="archive-meta-row" key={r.id}>
                            <div>
                              <strong>
                                {platforms.find(
                                  (p) => p.platform.id === r.platformId,
                                )?.platform.name ?? r.platformId}
                              </strong>
                              <p>
                                {[r.region, r.languages, r.titleId]
                                  .filter(Boolean)
                                  .join(" · ")}
                              </p>
                            </div>
                            <button onClick={() => releaseForm(r)}>
                              {t("archive.edit")}
                            </button>
                          </div>
                        ))}
                      </section>
                    </div>
                    <aside className="content-card archive-facts">
                      <h3>{t("archive.details")}</h3>
                      <dl>
                        {["developer", "publisher", "genre", "year"].map(
                          (key) => (
                            <div key={key}>
                              <dt>{t(`archive.fields.${key}`)}</dt>
                              <dd>
                                {String(
                                  detail.card.game[key as keyof ArchiveGame] ||
                                    t("archive.unspecified"),
                                )}
                              </dd>
                            </div>
                          ),
                        )}
                      </dl>
                      <h3>{t("archive.collections")}</h3>
                      {overview?.collections.map((collection) => (
                        <label className="archive-check" key={collection.id}>
                          <input
                            type="checkbox"
                            disabled={busy}
                            checked={detail.collections.includes(collection.id)}
                            onChange={(e) =>
                              void action(() =>
                                archiveRequest({
                                  kind: "member",
                                  collectionId: collection.id,
                                  gameId,
                                  present: e.target.checked,
                                }),
                              )
                            }
                          />
                          {collection.name}
                        </label>
                      ))}
                      <button onClick={collectionForm}>
                        {t("archive.addCollection")}
                      </button>
                    </aside>
                  </div>
                )}
                {["versions", "updates", "dlcs"].includes(tab) && (
                  <>
                    <div className="archive-section-heading">
                      <div>
                        <h2>{t(`archive.tabs.${tab}`)}</h2>
                        <p>{t("archive.compatibilityHint")}</p>
                      </div>
                      <div className="actions">
                        {tab === "versions" && (
                          <button
                            disabled={detail.releases.length === 0}
                            onClick={() => editionForm()}
                          >
                            {t("archive.addEdition")}
                          </button>
                        )}
                        <button
                          disabled={
                            detail.editions.length === 0 ||
                            (tab !== "versions" &&
                              !detail.contents.some(
                                (c) => c.kind === "version",
                              ))
                          }
                          onClick={() =>
                            contentForm(
                              tab === "versions"
                                ? "version"
                                : tab === "updates"
                                  ? "update"
                                  : "dlc",
                            )
                          }
                        >
                          {t(
                            tab === "versions"
                              ? "archive.addVersion"
                              : tab === "updates"
                                ? "archive.addUpdate"
                                : "archive.addDlc",
                          )}
                        </button>
                      </div>
                    </div>
                    {tab === "versions" &&
                      detail.editions.map((e) => (
                        <div className="archive-edition" key={e.id}>
                          <span>
                            {editionLabel(e.id)} · {e.medium}
                          </span>
                          <button onClick={() => editionForm(e)}>
                            {t("archive.editEdition")}
                          </button>
                        </div>
                      ))}
                    {detail.contents.filter(
                      (c) =>
                        c.kind ===
                        (tab === "versions"
                          ? "version"
                          : tab === "updates"
                            ? "update"
                            : "dlc"),
                    ).length === 0 && (
                      <EmptyState
                        title="archive.noContents"
                        description="archive.noContentsHint"
                      />
                    )}
                    {detail.contents
                      .filter(
                        (c) =>
                          c.kind ===
                          (tab === "versions"
                            ? "version"
                            : tab === "updates"
                              ? "update"
                              : "dlc"),
                      )
                      .slice(0, contentLimit)
                      .map((content) => {
                        const Component =
                          tab === "dlcs"
                            ? DlcCard
                            : tab === "updates"
                              ? UpdateCard
                              : VersionCard;
                        return (
                          <Component
                            key={content.id}
                            content={content}
                            edition={editionLabel(content.editionId)}
                            image={
                              detail.assets.find(
                                (a) => a.contentId === content.id,
                              )?.id
                            }
                          >
                            <div className="archive-content-side">
                              {content.baseId && (
                                <p>
                                  {t("archive.compatibleWith")}{" "}
                                  <strong>
                                    {
                                      detail.contents.find(
                                        (c) => c.id === content.baseId,
                                      )?.name
                                    }{" "}
                                    ·{" "}
                                    {
                                      detail.contents.find(
                                        (c) => c.id === content.baseId,
                                      )?.version
                                    }
                                  </strong>
                                </p>
                              )}
                              {detail.dependencies
                                .filter((d) => d.contentId === content.id)
                                .map((d) => (
                                  <p key={d.requiredId}>
                                    {t("archive.requires")}{" "}
                                    {
                                      detail.contents.find(
                                        (c) => c.id === d.requiredId,
                                      )?.name
                                    }
                                  </p>
                                ))}
                              <p>
                                {t("archive.associatedFiles", {
                                  count: detail.files.filter(
                                    (f) => f.contentId === content.id,
                                  ).length,
                                })}
                              </p>
                              <div className="actions">
                                <button
                                  onClick={() =>
                                    contentForm(content.kind, content)
                                  }
                                >
                                  {t("archive.edit")}
                                </button>
                                <button onClick={() => dependencyForm(content)}>
                                  {t("archive.addDependency")}
                                </button>
                                {content.kind === "dlc" && (
                                  <button
                                    onClick={() =>
                                      assetForm("content", content.id)
                                    }
                                  >
                                    {t("archive.addArtwork")}
                                  </button>
                                )}
                                <button
                                  onClick={() => {
                                    setLinkOpen(true);
                                    setLinkContent(content.id);
                                    setTab("files");
                                  }}
                                >
                                  {t("archive.linkFiles")}
                                </button>
                              </div>
                            </div>
                          </Component>
                        );
                      })}
                    {detail.contents.filter(
                      (c) =>
                        c.kind ===
                        (tab === "versions"
                          ? "version"
                          : tab === "updates"
                            ? "update"
                            : "dlc"),
                    ).length > contentLimit && (
                      <button onClick={() => setContentLimit((n) => n + 24)}>
                        {t("archive.showMore")}
                      </button>
                    )}
                  </>
                )}
                {tab === "files" && (
                  <>
                    <div className="archive-section-heading">
                      <div>
                        <h2>{t("archive.tabs.files")}</h2>
                        <p>{t("archive.originalsHint")}</p>
                      </div>
                      <div className="actions">
                        <button
                          disabled={detail.contents.length === 0}
                          onClick={() => {
                            setLinkOpen(!linkOpen);
                            setLinkContent(detail.contents[0]?.id ?? "");
                          }}
                        >
                          {t("archive.linkFiles")}
                        </button>
                        <button
                          disabled={detail.contents.length === 0}
                          onClick={attachForm}
                        >
                          {t("archive.attachEver")}
                        </button>
                        <button
                          disabled={detail.contents.length === 0}
                          onClick={() => sourceForm()}
                        >
                          {t("archive.addSource")}
                        </button>
                        <button onClick={() => onNavigate("library")}>
                          {t("archive.importOriginals")}
                        </button>
                      </div>
                    </div>
                    {linkOpen && (
                      <section className="content-card">
                        <h3>{t("archive.linkFiles")}</h3>
                        <p>{t("archive.linkHint")}</p>
                        <label>
                          {t("archive.fields.contentId")}
                          <select
                            value={linkContent}
                            onChange={(e) => setLinkContent(e.target.value)}
                          >
                            {detail.contents.map((c) => (
                              <option key={c.id} value={c.id}>
                                {c.name} · {editionLabel(c.editionId)}
                              </option>
                            ))}
                          </select>
                        </label>
                        <label>
                          {t("archive.findLocal")}
                          <input
                            type="search"
                            value={localSearch}
                            onChange={(e) => setLocalSearch(e.target.value)}
                          />
                        </label>
                        {localError !== undefined && (
                          <ErrorNotice
                            error={localError}
                            onRetry={() => void readLocals()}
                          />
                        )}
                        {localLoading && (
                          <p role="status">{t("archive.loading")}</p>
                        )}
                        {!localLoading &&
                          localLoaded &&
                          localItems.length === 0 && (
                            <p>{t("archive.noLocalItems")}</p>
                          )}
                        <div className="archive-local-list">
                          {localItems.map((item) => (
                            <div key={item.id} className="archive-meta-row">
                              <span>{item.title}</span>
                              <div className="actions">
                                <button
                                  disabled={busy || !linkContent}
                                  onClick={() =>
                                    void action(() =>
                                      archiveRequest({
                                        kind: "linkItem",
                                        contentId: linkContent,
                                        itemId: item.id,
                                      }),
                                    )
                                  }
                                >
                                  {t("archive.link")}
                                </button>
                                <button
                                  disabled={busy}
                                  onClick={() => onNavigate("share", item)}
                                >
                                  {t("archive.share")}
                                </button>
                                <button
                                  disabled={busy}
                                  onClick={() =>
                                    void action(
                                      () => exportEver({ itemId: item.id }),
                                      "notifications.referencesSaved",
                                    )
                                  }
                                >
                                  {t("archive.createEver")}
                                </button>
                              </div>
                            </div>
                          ))}
                        </div>
                        <div className="actions">
                          <button
                            disabled={localLoading}
                            onClick={() => void readLocals()}
                          >
                            {t("archive.firstPage")}
                          </button>
                          {localCursor && (
                            <button
                              disabled={localLoading}
                              onClick={() => void readLocals(localCursor)}
                            >
                              {t("archive.next")}
                            </button>
                          )}
                          <button onClick={() => setLinkOpen(false)}>
                            {t("archive.close")}
                          </button>
                        </div>
                      </section>
                    )}
                    {detail.files.length === 0 && (
                      <EmptyState
                        title="archive.noFiles"
                        description="archive.noFilesHint"
                      />
                    )}
                    <div className="archive-file-list">
                      {detail.files.slice(0, contentLimit).map((file) => (
                        <article className="content-card" key={file.id}>
                          <div className="archive-section-heading">
                            <h3>{file.name}</h3>
                            <PreservationBadge
                              state={
                                file.itemId
                                  ? preservationState(
                                      detail.states.filter(
                                        (s) => s.fileId === file.id,
                                      ),
                                    )
                                  : "unknown"
                              }
                            />
                          </div>
                          <p>
                            {
                              detail.contents.find(
                                (c) => c.id === file.contentId,
                              )?.name
                            }{" "}
                            · {file.format} ·{" "}
                            {formatBytes(file.sizeBytes, i18n.language)}
                          </p>
                          {file.sha256 && (
                            <code className="archive-hash">
                              SHA-256 {file.sha256}
                            </code>
                          )}
                          {detail.states
                            .filter((s) => s.fileId === file.id)
                            .map((s, i) => (
                              <p className="archive-file-path" key={i}>
                                {s.location}
                              </p>
                            ))}
                          {file.itemId && (
                            <button
                              disabled={busy}
                              onClick={() => void verify(file.itemId!)}
                            >
                              {t("archive.verifyFiles")}
                            </button>
                          )}
                        </article>
                      ))}
                    </div>
                    {detail.files.length > contentLimit && (
                      <button onClick={() => setContentLimit((n) => n + 24)}>
                        {t("archive.showMore")}
                      </button>
                    )}
                    <ArchiveLiveTransfers sources={detail.sources} />
                    <h3>{t("archive.distribution")}</h3>
                    <p>{t("archive.networkUnknown")}</p>
                    {detail.sources.map((source) => (
                      <DownloadPanel key={source.id}>
                        <div>
                          <strong>
                            {
                              detail.contents.find(
                                (c) => c.id === source.contentId,
                              )?.name
                            }
                          </strong>
                          <p>
                            {source.license} · {source.attribution}
                          </p>
                          <PreservationBadge state="reference" />
                        </div>
                        <div className="actions">
                          <button onClick={() => sourceForm(source)}>
                            {t("archive.edit")}
                          </button>
                          <button
                            onClick={() => {
                              setDownloadSource(source);
                              setRights(false);
                            }}
                          >
                            {t("archive.reviewDownload")}
                          </button>
                        </div>
                      </DownloadPanel>
                    ))}
                    {downloadSource && (
                      <section className="content-card">
                        <h3>{t("archive.reviewDownload")}</h3>
                        <p>{t("archive.downloadHint")}</p>
                        <p>{downloadSource.license}</p>
                        <label className="archive-check">
                          <input
                            type="checkbox"
                            checked={rights}
                            onChange={(e) => setRights(e.target.checked)}
                          />
                          {t("archive.rightsConfirmation")}
                        </label>
                        <div className="actions">
                          <button onClick={() => setDownloadSource(undefined)}>
                            {t("archive.cancel")}
                          </button>
                          <button
                            className="primary"
                            disabled={!rights}
                            onClick={() =>
                              onNavigate(
                                "download",
                                undefined,
                                downloadSource.magnet,
                              )
                            }
                          >
                            {t("archive.continueDownload")}
                          </button>
                        </div>
                      </section>
                    )}
                  </>
                )}
                {tab === "preservation" && (
                  <>
                    <section className="content-card">
                      <div className="archive-section-heading">
                        <h2>{t("archive.tabs.preservation")}</h2>
                        <PreservationBadge
                          state={preservationState(
                            detail.states,
                            detail.files.map((f) => f.id),
                          )}
                        />
                      </div>
                      <p>{t("archive.preservationHint")}</p>
                      <dl className="archive-stat-row">
                        <div>
                          <dt>{t("archive.localCopies")}</dt>
                          <dd>
                            {
                              detail.states.filter(
                                (s) => s.status === "verified",
                              ).length
                            }
                          </dd>
                        </div>
                        <div>
                          <dt>{t("archive.networkAvailability")}</dt>
                          <dd>{t("archive.unknown")}</dd>
                        </div>
                        <div>
                          <dt>{t("archive.seeders")}</dt>
                          <dd>—</dd>
                        </div>
                      </dl>
                      <button onClick={() => onNavigate("transfers")}>
                        {t("archive.liveTransfers")}
                      </button>
                    </section>
                    <ArchiveLiveTransfers sources={detail.sources} />
                    <section className="content-card">
                      <h3>{t("archive.verificationObservations")}</h3>
                      {detail.states.length === 0 && (
                        <p>{t("archive.noVerification")}</p>
                      )}
                      {detail.states.slice(0, contentLimit).map((state, i) => (
                        <div className="archive-meta-row" key={i}>
                          <span>
                            {
                              detail.files.find((f) => f.id === state.fileId)
                                ?.name
                            }
                            <small>
                              {state.checkedAt ?? t("archive.notChecked")}
                            </small>
                          </span>
                          <PreservationBadge
                            state={
                              state.status === "verified"
                                ? "verified"
                                : [
                                      "changed",
                                      "missing",
                                      "inaccessible",
                                    ].includes(state.status)
                                  ? "atRisk"
                                  : "unknown"
                            }
                          />
                        </div>
                      ))}
                      <p>{t("archive.historyScope")}</p>
                      <button onClick={() => onNavigate("tasks")}>
                        {t("archive.openActivity")}
                      </button>
                    </section>
                  </>
                )}
                {tab === "community" && (
                  <section className="content-card">
                    <div className="archive-section-heading">
                      <h2>{t("archive.sourcesAndHistory")}</h2>
                      <button onClick={provenanceForm}>
                        {t("archive.addAttribution")}
                      </button>
                    </div>
                    {detail.catalogs.map((c) => (
                      <div key={c.id} className="archive-meta-row">
                        <strong>{c.name}</strong>
                        <p>{c.attribution}</p>
                        <small>{c.importedAt}</small>
                      </div>
                    ))}
                    {detail.provenance.slice(0, contentLimit).map((source) => (
                      <article className="archive-provenance" key={source.id}>
                        <strong>{source.source}</strong>
                        <p>{source.note}</p>
                        <small>
                          {source.attribution} · {source.createdAt}
                        </small>
                      </article>
                    ))}
                    {detail.provenance.length > contentLimit && (
                      <button onClick={() => setContentLimit((n) => n + 24)}>
                        {t("archive.showMore")}
                      </button>
                    )}
                    <p>{t("archive.communityLocal")}</p>
                  </section>
                )}
              </section>
            </>
          )
        )
      ) : (
        <>
          {view === "archive" && !query.platformId && !query.search && (
            <>
              <section className="archive-intro">
                <div>
                  <p className="eyebrow">
                    PRESERVE THE PAST. SHARE THE FUTURE.
                  </p>
                  <h2>{t("archive.heroTitle")}</h2>
                  <p>{t("archive.heroDescription")}</p>
                </div>
                <span aria-hidden="true">∞</span>
              </section>
              {overview && (
                <div className="archive-stats">
                  {(
                    [
                      ["games", overview.stats.games],
                      ["platforms", overview.stats.platforms],
                      ["localFiles", overview.stats.localFiles],
                      ["catalogs", overview.stats.catalogs],
                    ] as const
                  ).map(([key, value]) => (
                    <div key={key}>
                      <strong>{value.toLocaleString()}</strong>
                      <span>{t(`archive.metrics.${key}`)}</span>
                    </div>
                  ))}
                </div>
              )}
              <div className="archive-section-heading">
                <h2>{t("archive.featuredPlatforms")}</h2>
                <button onClick={() => onNavigate("platforms")}>
                  {t("archive.allPlatforms")}
                </button>
              </div>
              {platformGrid(
                platforms
                  .filter(
                    (p, i, all) =>
                      all.findIndex(
                        (q) =>
                          q.platform.manufacturer === p.platform.manufacturer,
                      ) === i,
                  )
                  .slice(0, 4),
              )}
              {overview && overview.recent.length > 0 && (
                <>
                  <h2>{t("archive.recentlyAdded")}</h2>
                  {grid(overview.recent)}
                  <h2>{t("archive.recentlyUpdated")}</h2>
                  {grid(overview.updated)}
                </>
              )}
            </>
          )}
          {view === "platforms" && !query.platformId ? (
            <>
              <div className="archive-section-heading">
                <h2>{t("archive.explorePlatforms")}</h2>
                <span>{t("archive.platformIntro")}</span>
              </div>
              <label className="archive-search">
                {t("archive.findPlatform")}
                <input
                  type="search"
                  value={query.search}
                  onChange={(e) => filters({ search: e.target.value })}
                />
              </label>
              {!overview && loading ? (
                <ArchiveSkeleton />
              ) : (
                platformGrid(
                  platforms.filter((p) =>
                    `${p.platform.name} ${p.platform.manufacturer}`
                      .toLocaleLowerCase()
                      .includes(query.search.toLocaleLowerCase()),
                  ),
                )
              )}
            </>
          ) : (
            <>
              {platform && (
                <section className="archive-console-hero">
                  <div className="archive-console-image">
                    <PlatformCard
                      item={platform}
                      onOpen={() => {
                        setQuery((q) => ({ ...q, search: "" }));
                      }}
                    />
                  </div>
                  <div>
                    <p className="eyebrow">
                      {platform.platform.manufacturer} ·{" "}
                      {platform.platform.year ?? t("archive.unknownYear")}
                    </p>
                    <h2>{platform.platform.name}</h2>
                    <p className="archive-prose">
                      {platform.platform.description ||
                        t("archive.noPlatformHistory")}
                    </p>
                    <p>
                      {t("archive.gamesCount", { count: platform.games })} ·{" "}
                      {t("archive.localCount", { count: platform.withFiles })}
                    </p>
                    <p>{t("archive.preservationUnknown")}</p>
                    <div className="actions">
                      <button onClick={() => platformForm(platform.platform)}>
                        {t("archive.editPlatform")}
                      </button>
                      <button
                        onClick={() =>
                          assetForm("platform", platform.platform.id)
                        }
                      >
                        {t("archive.addArtwork")}
                      </button>
                    </div>
                  </div>
                </section>
              )}
              {view === "community" && (
                <section className="content-card">
                  <h2>{t("archive.communityCatalogs")}</h2>
                  <p>{t("archive.communityLocal")}</p>
                  <p>{t("archive.catalogTrust")}</p>
                  <div className="archive-collection-chips">
                    {overview?.catalogs.map((c) => (
                      <button
                        key={c.id}
                        aria-pressed={query.catalogId === c.id}
                        onClick={() =>
                          filters({
                            catalogId: query.catalogId === c.id ? "" : c.id,
                          })
                        }
                      >
                        {c.name}
                        <small>{c.attribution}</small>
                      </button>
                    ))}
                  </div>
                  {overview?.catalogs.length === 0 && (
                    <p>{t("archive.noCatalogs")}</p>
                  )}
                </section>
              )}
              {view === "collections" && (
                <section className="content-card">
                  <div className="archive-section-heading">
                    <h2>{t("archive.collections")}</h2>
                    <button onClick={collectionForm}>
                      {t("archive.addCollection")}
                    </button>
                  </div>
                  <div className="archive-collection-chips">
                    {["favorites", "recent", "personal"].map((scope) => (
                      <button
                        key={scope}
                        aria-pressed={query.scope === scope}
                        onClick={() =>
                          filters({
                            scope: query.scope === scope ? "" : scope,
                            collectionId: "",
                            order: scope === "recent" ? "visited" : "name",
                          })
                        }
                      >
                        {t(`archive.scopes.${scope}`)}
                      </button>
                    ))}
                    {overview?.collections.map((c) => (
                      <button
                        key={c.id}
                        aria-pressed={query.collectionId === c.id}
                        onClick={() =>
                          filters({
                            collectionId:
                              query.collectionId === c.id ? "" : c.id,
                            scope: "",
                          })
                        }
                      >
                        {c.name}
                      </button>
                    ))}
                  </div>
                  <p>{t("archive.collectionsHint")}</p>
                </section>
              )}
              {view === "preservation" && (
                <section className="content-card">
                  <h2>{t("archive.preservationOverview")}</h2>
                  <p>{t("archive.preservationHint")}</p>
                  <p>{t("archive.networkUnknown")}</p>
                  <div className="actions">
                    <button onClick={() => filters({ availability: "local" })}>
                      {t("archive.localFilter")}
                    </button>
                    <button onClick={() => onNavigate("library")}>
                      {t("archive.openLocal")}
                    </button>
                    <button onClick={() => onNavigate("transfers")}>
                      {t("archive.liveTransfers")}
                    </button>
                  </div>
                </section>
              )}
              <section className="archive-explorer">
                <div className="archive-section-heading">
                  <h2>{t("archive.exploreGames")}</h2>
                  <div
                    className="archive-view-switch"
                    role="group"
                    aria-label={t("archive.viewMode")}
                  >
                    {["grid", "compact", "list"].map((mode) => (
                      <button
                        key={mode}
                        aria-pressed={layout === mode}
                        onClick={() => {
                          setLayout(mode);
                          savePreference("layout", mode);
                        }}
                      >
                        {t(`archive.views.${mode}`)}
                      </button>
                    ))}
                  </div>
                </div>
                <div className="archive-filters">
                  <label className="archive-search">
                    {t("archive.search")}
                    <input
                      type="search"
                      value={query.search}
                      placeholder={t("archive.searchPlaceholder")}
                      onChange={(e) => filters({ search: e.target.value })}
                    />
                  </label>
                  <label>
                    {t("archive.fields.platformId")}
                    <select
                      value={query.platformId}
                      onChange={(e) => filters({ platformId: e.target.value })}
                    >
                      <option value="">{t("archive.allPlatforms")}</option>
                      {platforms.map((p) => (
                        <option key={p.platform.id} value={p.platform.id}>
                          {p.platform.name}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label>
                    {t("archive.sort")}
                    <select
                      value={query.order}
                      onChange={(e) => {
                        filters({ order: e.target.value });
                        savePreference("order", e.target.value);
                      }}
                    >
                      {["name", "year", "recent", "updated"].map((order) => (
                        <option key={order} value={order}>
                          {t(`archive.sorts.${order}`)}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <details className="archive-more-filters">
                  <summary>{t("archive.moreFilters")}</summary>
                  <div className="archive-filters">
                    {["region", "genre", "language", "edition"].map((key) => (
                      <label key={key}>
                        {t(`archive.fields.${key}`)}
                        <input
                          value={String(query[key as keyof ArchiveSearch])}
                          onChange={(e) => filters({ [key]: e.target.value })}
                        />
                      </label>
                    ))}
                    <label>
                      {t("archive.fields.year")}
                      <input
                        type="number"
                        min="1950"
                        max="2200"
                        value={query.year ?? ""}
                        onChange={(e) =>
                          filters({
                            year: e.target.value
                              ? Number(e.target.value)
                              : null,
                          })
                        }
                      />
                    </label>
                    <label>
                      {t("archive.availability")}
                      <select
                        value={query.availability}
                        onChange={(e) =>
                          filters({ availability: e.target.value })
                        }
                      >
                        <option value="">{t("archive.anyAvailability")}</option>
                        <option value="local">
                          {t("archive.status.local")}
                        </option>
                        <option value="reference">
                          {t("archive.status.reference")}
                        </option>
                      </select>
                    </label>
                    <button onClick={() => setQuery({ ...initialQuery })}>
                      {t("archive.clearFilters")}
                    </button>
                  </div>
                </details>
                {loading ? (
                  <ArchiveSkeleton />
                ) : items.length > 0 ? (
                  <>
                    {grid(items)}
                    <div className="archive-pagination">
                      <span>{t("archive.results", { count: total })}</span>
                      <button
                        disabled={query.offset === 0}
                        onClick={() =>
                          setQuery((q) => ({
                            ...q,
                            offset: Math.max(0, q.offset - 48),
                          }))
                        }
                      >
                        {t("archive.back")}
                      </button>
                      <span>
                        {Math.floor(query.offset / 48) + 1} /{" "}
                        {Math.max(1, Math.ceil(total / 48))}
                      </span>
                      <button
                        disabled={query.offset + 48 >= total}
                        onClick={() =>
                          setQuery((q) => ({ ...q, offset: q.offset + 48 }))
                        }
                      >
                        {t("archive.next")}
                      </button>
                    </div>
                  </>
                ) : (
                  error === undefined && (
                    <EmptyState
                      title="archive.noGames"
                      description="archive.noGamesHint"
                      action="archive.addGame"
                      onAction={() => gameForm()}
                    />
                  )
                )}
              </section>
            </>
          )}
        </>
      )}
      {editor && (
        <ArchiveEditor
          key={editor.title}
          spec={editor}
          onClose={() => setEditor(undefined)}
        />
      )}
    </div>
  );
}
