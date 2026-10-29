import { useState } from "react";
import { useI18n } from "../lib/i18n";
import { useLibraryStore } from "../features/libraryStore";
import { chooseSettingsFile, saveUserFile } from "../features/files";
import { Dialog } from "../features/Dialog";
import { mergePack, PACK_LIMIT, parseLineupPack, sameLineup, type DuplicatePolicy, type LineupPack } from "./lineupPack";
export function LineupTransfer() {
  const { t } = useI18n(), items = useLibraryStore(s => s.items);
  const [preview, setPreview] = useState<LineupPack | null>(null), [policy, setPolicy] = useState<DuplicatePolicy>("skip"), [message, setMessage] = useState(""), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  const [name, setName] = useState("Katsu Lineups");
  async function load() { setBusy(true); setError(""); setMessage(""); try { const text = await chooseSettingsFile(PACK_LIMIT, "katsu lineups"); if (text !== null) setPreview(parseLineupPack(text)); } catch { setError(t("suite.packInvalid")); } finally { setBusy(false); } }
  async function output() { setBusy(true); setError(""); setMessage(""); try { const file = parseLineupPack(JSON.stringify({ format: "katsu-lineup-pack", version: 1, name, createdAt: new Date().toISOString(), items })); if (await saveUserFile("katsu-lineups.json", new TextEncoder().encode(JSON.stringify(file, null, 2)), "json")) setMessage(t("suite.packSaved")); } catch { setError(t("tools.fileError")); } finally { setBusy(false); } }
  function apply() { if (!preview) return; try { const result = mergePack(useLibraryStore.getState().items, preview.items, policy); useLibraryStore.setState({ items: result }); setPreview(null); setMessage(t("suite.packImported")); } catch { setError(t("suite.packCapacity")); } }
  return <section className="suite-transfer"><div className="personal-actions"><label className="suite-inline-field">{t("suite.packName")}<input maxLength={100} value={name} onChange={e => setName(e.target.value)}/></label><button disabled={busy} onClick={() => void load()}>{t("suite.importLineups")}</button><button disabled={busy || !items.length || !name.trim()} onClick={() => void output()}>{t("suite.exportLineups")}</button></div>{message && <p className="personal-success" role="status">{message}</p>}{error && <p className="personal-error" role="alert">{error}</p>}
    {preview && <Dialog title={t("suite.importLineups")} onClose={() => setPreview(null)}><h3>{preview.name}</h3><p>{t("suite.packPreview", { n: preview.items.length, maps: new Set(preview.items.map(v => v.map)).size, agents: new Set(preview.items.map(v => v.agent)).size, duplicates: preview.items.filter(v => items.some(old => sameLineup(old, v))).length })}</p><label className="suite-inline-field">{t("suite.duplicates")}<select value={policy} onChange={e => setPolicy(e.target.value as DuplicatePolicy)}><option value="skip">{t("suite.skip")}</option><option value="replace">{t("suite.replace")}</option><option value="both">{t("suite.keepBoth")}</option></select></label><div className="personal-actions"><button onClick={() => setPreview(null)}>{t("tools.cancel")}</button><button className="personal-primary" onClick={apply}>{t("tools.apply")}</button></div>{error && <p role="alert" className="personal-error">{error}</p>}</Dialog>}
  </section>;
}
