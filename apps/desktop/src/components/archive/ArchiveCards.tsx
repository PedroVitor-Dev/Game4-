import { useEffect, useRef, useState, type ReactNode } from "react";
import { useTranslation } from "react-i18next";
import type {
  ArchivePlatformCard,
  ArchiveGameCard,
  ArchiveContent,
  ArchiveAssetInfo,
} from "@game4/ipc-contracts";
import { archiveRequest, availability } from "../../services/archive";
const images = new Map<string, { url: string; refs: number; time: number }>();
const loading = new Map<string, Promise<string>>();
function trimImages() {
  for (const [key, image] of images) {
    if (images.size <= 48) break;
    if (image.refs === 0) {
      URL.revokeObjectURL(image.url);
      images.delete(key);
    }
  }
}
async function imageUrl(id: string): Promise<string> {
  const saved = images.get(id);
  if (saved) return saved.url;
  let pending = loading.get(id);
  if (!pending) {
    pending = archiveRequest({ kind: "asset", id })
      .then((r) => {
        if (r.kind !== "asset") throw Error("asset");
        const url = URL.createObjectURL(
          new Blob([new Uint8Array(r.bytes)], { type: r.mime }),
        );
        images.set(id, { url, refs: 0, time: Date.now() });
        return url;
      })
      .finally(() => loading.delete(id));
    loading.set(id, pending);
  }
  return pending;
}
export function ArchiveArtwork({
  id,
  title,
  platform = "",
  wide = false,
}: {
  id?: string | null;
  title: string;
  platform?: string;
  wide?: boolean;
}) {
  const ref = useRef<HTMLDivElement>(null);
  const [visible, setVisible] = useState(false);
  const [url, setUrl] = useState<string>();
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    setUrl(undefined);
    setFailed(false);
  }, [id]);
  useEffect(() => {
    if (!ref.current) return;
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((e) => e.isIntersecting)) {
          setVisible(true);
          observer.disconnect();
        }
      },
      { rootMargin: "200px" },
    );
    observer.observe(ref.current);
    return () => observer.disconnect();
  }, []);
  useEffect(() => {
    if (!visible || !id) return;
    let alive = true;
    let held = false;
    imageUrl(id)
      .then((url) => {
        if (alive) {
          const entry = images.get(id);
          if (entry) {
            entry.refs++;
            held = true;
          }
          setUrl(url);
        }
        trimImages();
      })
      .catch(() => {
        if (alive) setFailed(true);
      });
    return () => {
      alive = false;
      if (held) {
        const entry = images.get(id);
        if (entry) entry.refs = Math.max(0, entry.refs - 1);
      }
      trimImages();
    };
  }, [id, visible]);
  return (
    <div
      ref={ref}
      className={`archive-artwork ${wide ? "wide" : ""} ${id && !url && !failed ? "pending" : ""}`}
    >
      {url && !failed ? (
        <img
          src={url}
          alt={title}
          loading="lazy"
          decoding="async"
          onError={() => setFailed(true)}
        />
      ) : (
        <div className="archive-art-fallback" aria-label={title}>
          <span aria-hidden="true">∞</span>
          <strong>{title}</strong>
          <small>{platform || "GAME4∞"}</small>
        </div>
      )}
    </div>
  );
}
export function PreservationBadge({ state }: { state: string }) {
  const { t } = useTranslation();
  return (
    <span className={`archive-badge ${state}`}>
      {t(`archive.status.${state}`)}
    </span>
  );
}
export function FileAvailability({ card }: { card: ArchiveGameCard }) {
  return <PreservationBadge state={availability(card)} />;
}
export function PlatformCard({
  item,
  onOpen,
  onEdit,
}: {
  item: ArchivePlatformCard;
  onOpen: () => void;
  onEdit?: () => void;
}) {
  const { t } = useTranslation();
  return (
    <article className="archive-platform-card">
      <button className="archive-card-open" onClick={onOpen}>
        <ArchiveArtwork
          id={item.coverId}
          title={item.platform.name}
          platform={item.platform.manufacturer}
          wide
        />
        <div className="archive-card-body">
          <small>
            {item.platform.manufacturer} ·{" "}
            {item.platform.year ?? t("archive.unknownYear")}
          </small>
          <h3>{item.platform.name}</h3>
          <p>
            {t("archive.gamesCount", { count: item.games })}
            <span>{t("archive.localCount", { count: item.withFiles })}</span>
          </p>
        </div>
      </button>
      {onEdit && (
        <button className="archive-card-edit" onClick={onEdit}>
          {t("archive.edit")}
        </button>
      )}
    </article>
  );
}
export function GameCoverCard({
  item,
  onOpen,
  onFavorite,
}: {
  item: ArchiveGameCard;
  onOpen: () => void;
  onFavorite: () => void;
}) {
  const { t } = useTranslation();
  return (
    <article className="archive-game-card">
      <button className="archive-card-open" onClick={onOpen}>
        <ArchiveArtwork
          id={item.coverId}
          title={item.game.title}
          platform={item.platforms}
        />
        <div className="archive-card-body">
          <small>{item.platforms || t("archive.noRelease")}</small>
          <h3>{item.game.title}</h3>
          <p>
            {item.game.year ?? t("archive.unknownYear")}{" "}
            {item.region && `· ${item.region}`}
          </p>
          <FileAvailability card={item} />
          {(item.updates > 0 || item.dlcs > 0) && (
            <p className="archive-content-count">
              {t("archive.contentsCount", {
                updates: item.updates,
                dlcs: item.dlcs,
              })}
            </p>
          )}
        </div>
      </button>
      <button
        className="archive-favorite"
        aria-pressed={item.favorite}
        aria-label={t(
          item.favorite ? "archive.unfavorite" : "archive.favorite",
          { title: item.game.title },
        )}
        onClick={onFavorite}
      >
        {item.favorite ? "★" : "☆"}
      </button>
    </article>
  );
}
export function GameHero({
  card,
  banner,
  children,
}: {
  card: ArchiveGameCard;
  banner?: string;
  children: ReactNode;
}) {
  return (
    <section className="archive-game-hero">
      {banner && (
        <div className="archive-hero-backdrop">
          <ArchiveArtwork id={banner} title={card.game.title} wide />
        </div>
      )}
      <div className="archive-hero-cover">
        <ArchiveArtwork
          id={card.coverId}
          title={card.game.title}
          platform={card.platforms}
        />
      </div>
      <div className="archive-hero-content">
        <p className="eyebrow">GAME4∞ · GAME ARCHIVE</p>
        <h2>{card.game.title}</h2>
        <p>
          {card.platforms} {card.game.year && `· ${card.game.year}`}
        </p>
        <FileAvailability card={card} />
        <div className="actions">{children}</div>
      </div>
    </section>
  );
}
export function ArtworkGallery({
  assets,
  title,
}: {
  assets: ArchiveAssetInfo[];
  title: string;
}) {
  return (
    <div className="archive-gallery">
      {assets.map((asset) => (
        <figure key={asset.id}>
          <ArchiveArtwork id={asset.id} title={title} wide />
          <figcaption>
            {asset.attribution} · {asset.license}
            <small>{asset.source}</small>
          </figcaption>
        </figure>
      ))}
    </div>
  );
}
export function VersionCard({
  content,
  edition,
  children,
}: {
  content: ArchiveContent;
  edition: string;
  children?: ReactNode;
}) {
  return (
    <article className="archive-version content-card">
      <div>
        <small>{edition}</small>
        <h3>{content.name}</h3>
        <p>
          {[content.version, content.build, content.releaseDate, content.format]
            .filter(Boolean)
            .join(" · ")}
        </p>
        <p>{content.description}</p>
        {content.changelog && <pre>{content.changelog}</pre>}
      </div>
      {children}
    </article>
  );
}
export const UpdateCard = VersionCard;
export function DlcCard({
  content,
  edition,
  image,
  children,
}: {
  content: ArchiveContent;
  edition: string;
  image?: string;
  children?: ReactNode;
}) {
  return (
    <div className="archive-dlc">
      <ArchiveArtwork id={image} title={content.name} wide />
      <VersionCard content={content} edition={edition}>
        {children}
      </VersionCard>
    </div>
  );
}
export function DownloadPanel({ children }: { children: ReactNode }) {
  return <div className="archive-download-panel">{children}</div>;
}
export function ArchiveSkeleton() {
  return (
    <div className="archive-grid" aria-busy="true">
      {[0, 1, 2, 3, 4, 5].map((v) => (
        <div key={v} className="archive-skeleton" />
      ))}
    </div>
  );
}
