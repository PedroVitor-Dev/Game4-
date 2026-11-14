import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useNotifications } from "./Notifications";
export function DownloadLink({ value }: { value: string }) {
  const { t } = useTranslation();
  const { notify } = useNotifications();
  const input = useRef<HTMLTextAreaElement>(null);
  const [message, setMessage] = useState("");
  const id = useId();
  async function copy() {
    try {
      await navigator.clipboard.writeText(value);
      setMessage("transfers.linkCopied");
      notify({ key: "transfers.linkCopied", kind: "success" });
    } catch {
      input.current?.focus();
      input.current?.select();
      setMessage("transfers.copyManually");
    }
  }
  return (
    <div className="download-link">
      <label htmlFor={id}>{t("ever.magnet")}</label>
      <textarea id={id} readOnly ref={input} value={value} />
      <button onClick={() => void copy()}>{t("transfers.copyLink")}</button>
      <span role="status">{message && t(message)}</span>
    </div>
  );
}
