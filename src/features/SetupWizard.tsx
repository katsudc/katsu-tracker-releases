import { useEffect, useState } from "react";
import { availableMonitors, getCurrentWindow, PhysicalPosition, type Monitor } from "@tauri-apps/api/window";
import { Check, Monitor as MonitorIcon, Sparkles } from "lucide-react";
import { useSettingsStore, THEMES } from "../stores/settingsStore";
import { usePersonalStore } from "./personalStore";
import { useI18n } from "../lib/i18n";
import { SpikeOutputSettings } from "../components/SpikeOutputSettings";
import { Dialog } from "./Dialog";

export function SetupWizard({ onClose }: { onClose: () => void }) {
  const { t, locale, setLocale } = useI18n();
  const { theme, setTheme, hotkey, saveCurrentPosition, setWindowStyle } = useSettingsStore();
  const complete = usePersonalStore(s => s.completeSetup);
  const [step, setStep] = useState(0), [mode, setMode] = useState("borderless"), [monitor, setMonitor] = useState("");
  const [monitors, setMonitors] = useState<Monitor[]>([]), [error, setError] = useState(""), [busy, setBusy] = useState(false);
  useEffect(() => { let live = true; void availableMonitors().then(value => { if (live) setMonitors(value); }).catch(() => {}); return () => { live = false; }; }, []);
  function close() { if (busy) return; complete(); onClose(); }
  async function finish() {
    setBusy(true); setError("");
    try {
      if (monitor !== "" && monitors[Number(monitor)]) {
        const current = await availableMonitors(), selected = monitors[Number(monitor)];
        const display = current.find(m => m.position.x === selected.position.x && m.position.y === selected.position.y && m.size.width === selected.size.width && m.size.height === selected.size.height);
        if (!display) throw new Error("display");
        await setWindowStyle("free");
        const win = getCurrentWindow(), size = await win.outerSize();
        await win.setPosition(new PhysicalPosition(display.position.x + Math.max(0, Math.round((display.size.width - size.width) / 2)), display.position.y + Math.max(0, Math.round((display.size.height - size.height) / 2))));
        await saveCurrentPosition();
      }
      complete(); onClose();
    } catch { setError(t("setup.moveError")); } finally { setBusy(false); }
  }
  return <Dialog title={t("setup.title")} onClose={close}>
    <p className="personal-muted">{t("setup.description")}</p>
    <div className="setup-progress" aria-label={t("setup.step", { n: step + 1 })}>{[0, 1, 2].map(n => <i key={n} className={n <= step ? "is-active" : ""}/>)}</div>
    <div className="setup-step"><span className="eyebrow">{t("setup.step", { n: step + 1 })}</span>
      <h3>{step === 0 ? <Sparkles size={21}/> : step === 1 ? <MonitorIcon size={21}/> : <Check size={21}/>} {t(["setup.appearance", "setup.display", "setup.finish"][step])}</h3>
      {step === 0 && <div className="personal-form">
        <label>{t("settings.language")}<select value={locale} onChange={e => setLocale(e.target.value === "tr" ? "tr" : "en")}><option value="en">English</option><option value="tr">Türkçe</option></select></label>
        <div className="theme-picker"><span>{t("desktop.theme")}</span><div>{THEMES.map(v => <button key={v} data-theme-option={v} className={theme === v ? "selected" : ""} onClick={() => setTheme(v)} aria-pressed={theme === v} aria-label={t("theme." + v)}><i/>{theme === v && <Check size={13}/>}</button>)}</div></div>
      </div>}
      {step === 1 && <div className="personal-form">
        <label>{t("setup.mode")}<select value={mode} onChange={e => setMode(e.target.value)}><option value="borderless">{t("setup.borderless")}</option><option value="fullscreen">{t("setup.fullscreen")}</option></select><small>{t("setup.modeHint")}</small></label>
        <label>{t("setup.monitor")}<select value={monitor} onChange={e => setMonitor(e.target.value)}><option value="">{t("setup.automatic")}</option>{monitors.map((m, i) => <option key={i} value={i}>{t("setup.monitorOption", { n: i + 1, width: m.size.width, height: m.size.height })}</option>)}</select><small>{t("setup.follow")}</small></label>
      </div>}
      {step === 2 && <div className="setup-summary"><p>{t("setup.quick", { key: hotkey })}</p><p>{t(mode === "fullscreen" ? "setup.fullscreenHint" : "setup.desktopHint")}</p>{mode === "fullscreen" && <SpikeOutputSettings/>}</div>}
      {error && <p role="alert" className="personal-error">{error}</p>}
    </div>
    <div className="personal-actions setup-actions"><button disabled={busy} onClick={close}>{t("setup.later")}</button><span/>{step > 0 && <button disabled={busy} onClick={() => setStep(v => v - 1)}>{t("setup.back")}</button>}
      <button className="personal-primary" disabled={busy} onClick={() => step < 2 ? setStep(v => v + 1) : void finish()}>{t(step < 2 ? "setup.next" : "setup.done")}</button></div>
  </Dialog>;
}
