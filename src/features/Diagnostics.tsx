import { useEffect, useRef, useState } from "react";
import { invoke } from "../suite/ipc";
import { CheckCircle2, CircleHelp, RefreshCw, TriangleAlert } from "lucide-react";
import { useGameStore } from "../stores/gameStore";
import { useI18n } from "../lib/i18n";
import { diagnosticChecks, type DisplayProbe, type OutputProbe, type SpikeProbe } from "./diagnosticData";

export function Diagnostics({ onSpike }: { onSpike: () => void }) {
  const { t, locale } = useI18n();
  const connection = useGameStore(s => s.status);
  const reconnect = useGameStore(s => s.reconnect);
  const [probes, setProbes] = useState<{ output: OutputProbe | null; spike: SpikeProbe | null; display: DisplayProbe | null }>({ output: null, spike: null, display: null });
  const [busy, setBusy] = useState(false), [error, setError] = useState(false), [checked, setChecked] = useState<number | null>(null);
  const mounted = useRef(false), flight = useRef(false), language = useRef(locale);
  language.current = locale;
  async function refresh() {
    if (flight.current) return;
    flight.current = true; setBusy(true);
    const results = await Promise.allSettled([
      invoke<OutputProbe>("get_spike_output_status", { locale: language.current }),
      invoke<SpikeProbe>("spike_get_status"), invoke<DisplayProbe>("get_spike_editor_context"),
    ]);
    if (mounted.current) {
      const value = <T,>(i: number) => results[i].status === "fulfilled" ? results[i].value as T : null;
      setProbes({ output: value<OutputProbe>(0), spike: value<SpikeProbe>(1), display: value<DisplayProbe>(2) });
      setError(results.some(r => r.status === "rejected")); setChecked(Date.now()); setBusy(false);
    }
    flight.current = false;
  }
  useEffect(() => {
    mounted.current = true; void refresh();
    const timer = setInterval(() => { if (!document.hidden) void refresh(); }, 5000);
    return () => { mounted.current = false; clearInterval(timer); };
  }, []);
  const checks = diagnosticChecks(connection, probes.output, probes.spike, probes.display);
  return <section className="personal-card diagnostic-card">
    <div className="personal-heading"><div><h3>{t("diag.title")}</h3><p>{t("diag.description")}</p></div><button disabled={busy} onClick={() => void refresh()}><RefreshCw size={14} className={busy ? "animate-spin" : ""}/>{t("diag.refresh")}</button></div>
    <div className="diagnostic-grid">{checks.map(row => {
      const Icon = row.level === "ok" ? CheckCircle2 : row.level === "warn" ? TriangleAlert : CircleHelp;
      return <article key={row.id} className={"diagnostic-item is-" + row.level}><Icon size={18}/><div><h4>{t("diag." + row.id)}</h4><strong>{t(row.value)}</strong>{row.detail && <small>{row.detail}</small>}{row.hint && <p>{t(row.hint)}</p>}</div></article>;
    })}</div>
    <div className="personal-actions"><button onClick={onSpike}>{t("diag.spike")}</button><button disabled={connection === "CONNECTING" || connection === "RECONNECTING"} onClick={() => void reconnect()}>{t("diag.reconnect")}</button></div>
    {checked && <small className="personal-muted">{t("diag.checked", { time: new Date(checked).toLocaleTimeString(locale === "tr" ? "tr-TR" : "en-US") })}</small>}
    {error && <p className="personal-error" role="status">{t("diag.unavailable")}</p>}
  </section>;
}
