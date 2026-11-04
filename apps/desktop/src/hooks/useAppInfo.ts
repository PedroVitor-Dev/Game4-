import { useEffect, useState } from "react";
import { normalizeIpcError, type IpcError } from "@game4/ipc-contracts";
import { getAppInfo, isTauri } from "../services/desktop";

export type Runtime = "checking" | "connected" | "preview" | "unavailable";

export function useAppInfo() {
  const [runtime, setRuntime] = useState<Runtime>("checking");
  const [version, setVersion] = useState<string>();
  const [failure, setFailure] = useState<IpcError | null>(null);
  useEffect(() => {
    let active = true;
    if (!isTauri()) {
      setRuntime("preview");
      return;
    }
    getAppInfo()
      .then((info) => {
        if (active) {
          setVersion(info.version);
          setRuntime("connected");
        }
      })
      .catch((error: unknown) => {
        if (active) {
          setFailure(normalizeIpcError(error));
          setRuntime("unavailable");
        }
      });
    return () => {
      active = false;
    };
  }, []);
  return { runtime, version, failure };
}
