import { useEffect, useState } from "react";
import { getAppInfo, isTauri } from "../services/desktop";

export type Runtime = "checking" | "connected" | "preview" | "unavailable";

export function useAppInfo() {
  const [runtime, setRuntime] = useState<Runtime>("checking");
  const [version, setVersion] = useState<string>();
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
      .catch(() => {
        if (active) setRuntime("unavailable");
      });
    return () => {
      active = false;
    };
  }, []);
  return { runtime, version };
}
