import { useRuntimeStore } from "../suite/runtimeStore";
import { useConstantsStore } from "../stores/constantsStore";
import { useState } from "react";
import { Download, Upload } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useGameStore } from "../stores/gameStore";
import { parseSettingsFile, type SettingsFile } from "./preferenceData";
import { applyPreferences, capturePreferences } from "./preferenceTransfer";
import { chooseSettingsFile, saveUserFile } from "./files";

export function SettingsTransfer() {
  const { t } = useI18n();
  const resolveAgent = useConstantsStore(s => s.getAgentByUuid);
  const optional = useRuntimeStore(s => s.mode === "normal");
  const game = useGameStore(s => s.gameState.state);
  const [preview, setPreview] = useState<SettingsFile | null>(null);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const inGame = game === "ingame" || game === "pregame";
  async function exportFile() {
    setBusy(true); setMessage(""); setError("");
    try {
      const file = await capturePreferences();
      if (await saveUserFile("katsu-settings.json", new TextEncoder().encode(JSON.stringify(file, null, 2)), "json")) setMessage(t("tools.exported"));
    } catch { setError(t("tools.fileError")); } finally { setBusy(false); }
  }
  async function importFile() {
    setBusy(true); setMessage(""); setError("");
    try { const text = await chooseSettingsFile(); if (text !== null) setPreview(parseSettingsFile(text)); }
    catch { setError(t("tools.invalid")); } finally { setBusy(false); }
  }
  async function apply() {
    if (!preview || inGame) return;
    setBusy(true); setError("");
    try { await applyPreferences(preview); setPreview(null); setMessage(useI18n.getState().t("tools.imported")); }
    catch (e) { setError(t(e instanceof Error && e.message === "busy_game" ? "tools.busyGame" : e instanceof Error && e.message === "apply_restored" ? "tools.applyError" : "tools.applyPartial")); }
    finally { setBusy(false); }
  }
  return <section className="personal-card">
    <h3>{t("tools.transfer")}</h3><p>{t("tools.transferHint")}</p>
    <div className="personal-actions"><button disabled={busy || !optional} onClick={() => void exportFile()}><Download size={15}/>{t("tools.export")}</button><button disabled={busy || inGame || !optional} onClick={() => void importFile()}><Upload size={15}/>{t("tools.import")}</button></div>
    {!optional && <p className="personal-muted">{t("suite.safeTransfer")}</p>}
    {inGame && <p className="personal-muted">{t("tools.busyGame")}</p>}
    {preview && <div className="settings-import-preview">
      <h4>{t("tools.preview")}</h4><p>{t("tools.importHint")}</p>
      <dl>{[
        [t("settings.language"), preview.preferences.locale.toUpperCase()], [t("desktop.theme"), t("theme." + preview.preferences.theme)],
        [t("settings.hotkey"), preview.preferences.hotkey], [t("instalock.title"), (resolveAgent(preview.preferences.autoLockAgent ?? preview.preferences.pausedAutoLockAgent ?? "")?.name ?? "—") + " · " + preview.preferences.autoLockDelaySeconds + "s"],
        [t("settings.mapPrefs"), String(Object.keys(preview.preferences.mapAgentPreferences).length)],
        [t("spike.title"), t(preview.preferences.spikeEnabled ? "spike.on" : "spike.off") + " · " + preview.preferences.appearance.font_size + "px"],
        [t("spike.gameBarTitle"), preview.preferences.appearance.game_bar!.font_size + "px"],
        [t("stream.title"), t(preview.preferences.stream.enabled ? "spike.on" : "spike.off")],
      ].map(([key, value]) => <div key={key}><dt>{key}</dt><dd>{value}</dd></div>)}</dl>
      <div className="personal-actions"><button disabled={busy} onClick={() => setPreview(null)}>{t("tools.cancel")}</button><button className="personal-primary" disabled={busy || inGame || !optional} onClick={() => void apply()}>{t("tools.apply")}</button></div>
    </div>}
    {message && <p role="status" className="personal-success">{message}</p>}
    {error && <p role="alert" className="personal-error">{error}</p>}
  </section>;
}
