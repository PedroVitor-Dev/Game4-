import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useRef,
  useState,
  type ReactNode,
} from "react";
import { useTranslation } from "react-i18next";
import type {
  TaskKind,
  TaskSnapshot,
  TransferSnapshot,
} from "@game4/ipc-contracts";
import {
  getAppInfo,
  isTauri,
  listTasks,
  listTransfers,
} from "../services/desktop";
import {
  addNotice,
  observeTasks,
  observeTransfers,
  type Notice,
  type NoticeInput,
} from "../services/notifications";
import type { Navigate } from "../services/navigation";
const Context = createContext<{
  notices: Notice[];
  notify: (input: NoticeInput) => void;
  markRead: (id?: number) => void;
  watchTask: (id: string, kind: TaskKind) => void;
  watchTransfer: (item: TransferSnapshot) => void;
} | null>(null);
export function NotificationProvider({ children }: { children: ReactNode }) {
  const [notices, setNotices] = useState<Notice[]>([]);
  const sequence = useRef(0);
  const seen = useRef(new Set<string>());
  const tasks = useRef(
    new Map<string, Pick<TaskSnapshot, "status" | "kind">>(),
  );
  const transfers = useRef(
    new Map<string, Pick<TransferSnapshot, "status" | "isSeeding">>(),
  );
  const notify = useCallback((input: NoticeInput) => {
    if (input.dedupeKey) {
      if (seen.current.has(input.dedupeKey)) return;
      seen.current.add(input.dedupeKey);
      if (seen.current.size > 2000)
        seen.current.delete(seen.current.values().next().value!);
    }
    const id = ++sequence.current;
    setNotices((old) => addNotice(old, input, id));
  }, []);
  const markRead = useCallback(
    (id?: number) =>
      setNotices((old) =>
        old.map((item) =>
          id === undefined || item.id === id
            ? { ...item, unread: false }
            : item,
        ),
      ),
    [],
  );
  const watchTask = useCallback((id: string, kind: TaskKind) => {
    tasks.current.set(id, { kind, status: "queued" });
  }, []);
  const watchTransfer = useCallback((item: TransferSnapshot) => {
    transfers.current.set(item.id, { status: "resuming", isSeeding: false });
  }, []);
  useEffect(() => {
    if (!isTauri()) return;
    let active = true;
    let timer: ReturnType<typeof setTimeout>;
    void getAppInfo()
      .then((info) => {
        if (!active) return;
        const canTasks = info.capabilities.includes("tasks_list"),
          canTransfers = info.capabilities.includes("transfers_list");
        if (!canTasks && !canTransfers) return;
        async function poll() {
          const [jobs, sessions] = await Promise.allSettled([
            canTasks
              ? listTasks({ cursor: null, limit: 50 })
              : Promise.resolve(null),
            canTransfers ? listTransfers() : Promise.resolve(null),
          ]);
          if (!active) return;
          if (jobs.status === "fulfilled" && jobs.value)
            observeTasks(tasks.current, jobs.value.items).forEach(notify);
          if (sessions.status === "fulfilled" && sessions.value)
            observeTransfers(transfers.current, sessions.value.items).forEach(
              notify,
            );
          timer = setTimeout(() => void poll(), 2500);
        }
        void poll();
      })
      .catch(() => {});
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [notify]);
  return (
    <Context.Provider
      value={{ notices, notify, markRead, watchTask, watchTransfer }}
    >
      {children}
    </Context.Provider>
  );
}
export function useNotifications() {
  const value = useContext(Context);
  if (!value) throw new Error("Missing NotificationProvider");
  return value;
}
export function NotificationCenter({ onNavigate }: { onNavigate: Navigate }) {
  const { t } = useTranslation();
  const { notices, markRead } = useNotifications();
  const [open, setOpen] = useState(false);
  const container = useRef<HTMLDivElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const unread = notices.filter((item) => item.unread);
  useEffect(() => {
    if (!open) return;
    const close = (event: PointerEvent) => {
      if (!container.current?.contains(event.target as Node)) setOpen(false);
    };
    document.addEventListener("pointerdown", close);
    return () => document.removeEventListener("pointerdown", close);
  }, [open]);
  function show(item: Notice) {
    markRead(item.id);
    if (item.target) onNavigate(item.target);
    setOpen(false);
  }
  const card = (item: Notice) => (
    <article key={item.id} className={`notice notice-${item.kind ?? "info"}`}>
      <span className="notice-icon" aria-hidden="true">
        {item.kind === "error" ? "!" : item.kind === "success" ? "✓" : "i"}
      </span>
      <div>
        <p>{t(item.key, item.params)}</p>
        {item.target && (
          <button className="notice-action" onClick={() => show(item)}>
            {t(`notifications.view.${item.target}`)}
          </button>
        )}
      </div>
      {item.unread && (
        <button
          className="notice-dismiss"
          aria-label={t("notifications.dismiss")}
          onClick={() => markRead(item.id)}
        >
          ×
        </button>
      )}
    </article>
  );
  return (
    <div
      className="notification-center"
      ref={container}
      onKeyDown={(e) => {
        if (e.key === "Escape" && open) {
          e.preventDefault();
          setOpen(false);
          trigger.current?.focus();
        }
      }}
    >
      <button
        className="notification-trigger"
        ref={trigger}
        aria-expanded={open}
        aria-controls="notification-history"
        onClick={() => {
          if (!open) markRead();
          setOpen(!open);
        }}
      >
        {t("notifications.title")}{" "}
        {unread.length > 0 && <span>{unread.length}</span>}
      </button>
      {open && (
        <section
          id="notification-history"
          className="notification-history"
          aria-label={t("notifications.title")}
        >
          <div className="session-heading">
            <h2>{t("notifications.title")}</h2>
            <button
              aria-label={t("notifications.close")}
              onClick={() => {
                setOpen(false);
                trigger.current?.focus();
              }}
            >
              ×
            </button>
          </div>
          {notices.length ? (
            [...notices].reverse().map(card)
          ) : (
            <div className="notification-empty">
              <h3>{t("notifications.emptyTitle")}</h3>
              <p>{t("notifications.emptyText")}</p>
            </div>
          )}
          <p className="notification-footnote">{t("notifications.scope")}</p>
        </section>
      )}
      <div
        className="toast-region"
        aria-live="polite"
        aria-relevant="additions text"
        aria-atomic="false"
      >
        {!open && unread.slice(-3).map(card)}
      </div>
    </div>
  );
}
