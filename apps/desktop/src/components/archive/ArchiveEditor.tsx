import { useEffect, useRef, useState, type FormEvent } from "react";
import { useTranslation } from "react-i18next";
import { ErrorNotice } from "../Feedback";
export type FormValues = Record<string, string | boolean>;
export type FormField = {
  name: string;
  type?: "text" | "number" | "textarea" | "checkbox" | "date";
  required?: boolean;
  options?: { value: string; label: string }[];
  disabled?: boolean;
};
export type EditorSpec = {
  title: string;
  initial: FormValues;
  fields: FormField[] | ((values: FormValues) => FormField[]);
  submit: (values: FormValues) => Promise<void>;
};
export function ArchiveEditor({
  spec,
  onClose,
}: {
  spec: EditorSpec;
  onClose: () => void;
}) {
  const { t } = useTranslation();
  const dialog = useRef<HTMLDialogElement>(null);
  const [values, setValues] = useState(spec.initial);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>();
  const [step, setStep] = useState(0);
  const fields =
    typeof spec.fields === "function" ? spec.fields(values) : spec.fields;
  const steps =
    fields.length > 6 ? [fields.slice(0, 5), fields.slice(5)] : [fields];
  useEffect(() => {
    const previous = document.activeElement;
    dialog.current?.showModal();
    return () => {
      dialog.current?.close();
      if (previous instanceof HTMLElement) previous.focus();
    };
  }, []);
  async function submit(event: FormEvent) {
    event.preventDefault();
    if (step < steps.length - 1) {
      setStep(step + 1);
      return;
    }
    setBusy(true);
    setError(undefined);
    try {
      await spec.submit(values);
      onClose();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  return (
    <dialog
      ref={dialog}
      className="archive-editor"
      onCancel={(e) => {
        e.preventDefault();
        if (!busy) onClose();
      }}
    >
      <form onSubmit={submit}>
        <header>
          <div>
            <p className="eyebrow">{t("archive.localEditor")}</p>
            <h2>{t(spec.title)}</h2>
          </div>
          <button
            type="button"
            onClick={onClose}
            disabled={busy}
            aria-label={t("archive.close")}
          >
            ×
          </button>
        </header>
        <p>{t("archive.editorHint")}</p>
        {steps.length > 1 && (
          <p>{t("archive.step", { current: step + 1, total: steps.length })}</p>
        )}
        <div className="archive-form-grid">
          {steps[step].map((field) => (
            <label
              key={field.name}
              className={field.type === "textarea" ? "span-all" : ""}
            >
              {t(`archive.fields.${field.name}`)}
              {field.required && " *"}
              {field.options ? (
                <select
                  required={field.required}
                  disabled={busy || field.disabled}
                  value={String(values[field.name] ?? "")}
                  onChange={(e) =>
                    setValues((old) => ({
                      ...old,
                      [field.name]: e.target.value,
                      ...(field.name === "editionId" ? { baseId: "" } : {}),
                    }))
                  }
                >
                  <option value="">{t("archive.choose")}</option>
                  {field.options.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              ) : field.type === "textarea" ? (
                <textarea
                  maxLength={20000}
                  rows={4}
                  required={field.required}
                  disabled={busy || field.disabled}
                  value={String(values[field.name] ?? "")}
                  onChange={(e) =>
                    setValues((old) => ({
                      ...old,
                      [field.name]: e.target.value,
                    }))
                  }
                />
              ) : field.type === "checkbox" ? (
                <input
                  type="checkbox"
                  disabled={busy || field.disabled}
                  checked={values[field.name] === true}
                  onChange={(e) =>
                    setValues((old) => ({
                      ...old,
                      [field.name]: e.target.checked,
                    }))
                  }
                />
              ) : (
                <input
                  type={field.type ?? "text"}
                  min={field.type === "number" ? 1950 : undefined}
                  max={field.type === "number" ? 2200 : undefined}
                  maxLength={
                    field.name === "magnet"
                      ? 16384
                      : field.name === "license" ||
                          field.name === "source" ||
                          field.name === "attribution"
                        ? 2000
                        : 512
                  }
                  required={field.required}
                  disabled={busy || field.disabled}
                  value={String(values[field.name] ?? "")}
                  onChange={(e) =>
                    setValues((old) => ({
                      ...old,
                      [field.name]: e.target.value,
                    }))
                  }
                />
              )}
            </label>
          ))}
        </div>
        {error !== undefined && (
          <ErrorNotice error={error} onDismiss={() => setError(undefined)} />
        )}
        <footer className="actions">
          {step > 0 && (
            <button
              type="button"
              onClick={() => setStep(step - 1)}
              disabled={busy}
            >
              {t("archive.back")}
            </button>
          )}
          <button type="button" onClick={onClose} disabled={busy}>
            {t("archive.cancel")}
          </button>
          <button className="primary" type="submit" disabled={busy}>
            {busy
              ? t("archive.saving")
              : t(step < steps.length - 1 ? "archive.next" : "archive.save")}
          </button>
        </footer>
      </form>
    </dialog>
  );
}
