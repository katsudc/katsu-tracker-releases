import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useI18n } from "../lib/i18n";

type OutputStatus = {
  enabled: boolean;
  mode: "fullscreen" | "borderless" | "windowed" | "unknown";
  output: string;
  game_bar_installed: boolean;
  widget_installed: boolean;
  pinned: boolean;
  click_through: boolean;
  phase: string;
  error: string | null;
};

function useOutputStatus() {
  const { locale } = useI18n();
  const [status, setStatus] = useState<OutputStatus | null>(null);
  const [error, setError] = useState(false);
  useEffect(() => {
    let disposed = false, changed = false;
    let off: (() => void) | undefined;
    void listen<OutputStatus>("spike-output-status", event => {
      changed = true;
      if (!disposed) setStatus(event.payload);
    }).then(async unlisten => {
      if (disposed) { unlisten(); return; }
      off = unlisten;
      const value = await invoke<OutputStatus>("get_spike_output_status", { locale });
      if (!disposed && !changed && value) setStatus(value);
    }).catch(() => { if (!disposed) setError(true); });
    return () => { disposed = true; off?.(); };
  }, [locale]);
  return { status, error, setError };
}

function SetupActions({ status, onDefer }: { status: OutputStatus; onDefer?: () => void }) {
  const { t } = useI18n();
  const [error, setError] = useState(false);
  const installing = status.phase === "installing_game_bar" || status.phase === "installing_widget";
  const ready = status.widget_installed && status.game_bar_installed;
  async function install() {
    setError(false);
    try {
      // This click is the installation and publisher-trust consent. Mounting,
      // dismissing, or receiving a display-mode event must never start a download.
      await invoke("prepare_spike_game_bar", { trustPublisher: true });
    } catch { setError(true); }
  }
  return <>
    {installing ? <p role="status">{t(status.phase === "installing_game_bar" ? "spike.installingGameBar" : "spike.installingWidget")}</p> : !ready ? <>
      <p>{t(status.game_bar_installed ? "spike.widgetInstallPrompt" : "spike.fullscreenInstallPrompt")}</p>
      <p className="spike-warning">{t("spike.trustExplanation")}</p>
      <div className="spike-output-actions">
        <button onClick={() => void install()}>{t("spike.installApprove")}</button>
        {onDefer && <button onClick={onDefer}>{t("spike.installDefer")}</button>}
      </div>
    </> : null}
    {(status.error === "store_required" || status.error === "game_bar_install") && <button onClick={() => void invoke("open_spike_game_bar", { store: true }).catch(() => setError(true))}>{t("spike.openStore")}</button>}
    {(error || (status.error && status.error !== "publisher_trust_required")) && <p role="alert" className="spike-warning">{t(
      status.error === "setup_cancelled" ? "spike.setupCancelled" :
      status.error === "administrator_required" ? "spike.administratorRequired" :
      status.error === "widget_package_invalid" ? "spike.widgetPackageInvalid" :
      status.error === "widget_package_missing" ? "spike.widgetPackageMissing" :
      status.error === "store_required" || status.error === "game_bar_install" ? "spike.gameBarInstallError" : "spike.outputSetupError"
    )}</p>}
  </>;
}

export function GameBarSetupNotice() {
  const { t } = useI18n();
  const { status } = useOutputStatus();
  const [deferred, setDeferred] = useState(false);
  if (!status || !status.enabled || status.mode !== "fullscreen" || deferred || status.phase === "checking" || (status.widget_installed && status.game_bar_installed)) return null;
  return <aside className="gamebar-setup-notice" aria-label={t("spike.setupTitle")}>
    <strong>{t("spike.setupTitle")}</strong>
    <SetupActions status={status} onDefer={() => setDeferred(true)} />
  </aside>;
}

export function SpikeOutputSettings() {
  const { t } = useI18n();
  const { status, error, setError } = useOutputStatus();
  if (!status) return error ? <p className="spike-warning">{t("spike.outputUnavailable")}</p> : null;
  const ready = status.widget_installed && status.game_bar_installed;
  return <div className="spike-output-settings">
    <div className="spike-output-line"><span>{t("spike.output")}</span><strong>{t(status.output === "game_bar" ? "spike.outputGameBar" : "spike.outputDesktop")}</strong></div>
    <p>{t("spike.outputAutomatic")}</p>
    <SetupActions status={status} />
    {ready && <>
      {status.pinned ? <span className="spike-output-ready">{t("spike.fullscreenReady")}</span> : <p>{t("spike.pinInstructions")}</p>}
      <div className="spike-output-line"><span>{t("spike.passThrough")}</span><strong className={status.click_through ? "spike-output-ready" : "spike-warning"}>{t(status.click_through ? "spike.on" : "spike.off")}</strong></div>
      {!status.click_through && <p className="spike-warning">{t("spike.passThroughWarning")}</p>}
      <button onClick={() => void invoke("open_spike_game_bar").catch(() => setError(true))}>{t("spike.openGameBar")}</button>
    </>}
    {error && <p role="alert" className="spike-warning">{t("spike.outputSetupError")}</p>}
  </div>;
}
