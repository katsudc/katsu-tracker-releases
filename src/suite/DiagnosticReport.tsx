import { useState } from "react";
import { useI18n } from "../lib/i18n";
import { invoke } from "./ipc";
import { saveUserFile } from "../features/files";
import { Dialog } from "../features/Dialog";
export function DiagnosticReport() {
  const { t } = useI18n(); const [preview, setPreview] = useState<unknown>(null), [busy, setBusy] = useState(false), [message, setMessage] = useState(""), [error, setError] = useState(false);
  async function create() { setBusy(true); setError(false); try { setPreview(await invoke("diagnostic_preview")); } catch { setError(true); } finally { setBusy(false); } }
  async function save() { setBusy(true); setError(false); try { const raw = await invoke<ArrayBuffer | number[]>("diagnostic_zip"); if (await saveUserFile("katsu-diagnostics.zip", new Uint8Array(raw), "zip")) { setMessage(t("suite.reportSaved")); setPreview(null); } } catch { setError(true); } finally { setBusy(false); } }
  return <section className="personal-card"><h3>{t("suite.createReport")}</h3><p>{t("suite.reportHint")}</p><div className="personal-actions"><button disabled={busy} onClick={() => void create()}>{t("suite.createReport")}</button></div>{message && <p className="personal-success">{message}</p>}{error && <p role="alert" className="personal-error">{t("suite.reportError")}</p>}{preview !== null && <Dialog title={t("suite.reportPreview")} onClose={() => { if (!busy) setPreview(null); }} wide><p>{t("suite.reportHint")}</p><pre className="suite-diagnostic-preview">{JSON.stringify(preview, null, 2)}</pre><div className="personal-actions"><button disabled={busy} onClick={() => void save()}>{t("suite.saveReport")}</button></div>{error && <p role="alert" className="personal-error">{t("suite.reportError")}</p>}</Dialog>}</section>;
}
