import { useEffect, useMemo, useState } from "react";
import { BookOpen, ExternalLink, LoaderCircle, Maximize2, Minimize2, Play, RefreshCw } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "../lib/i18n";
import { AGENTS } from "../lib/constants";
import { useConstantsStore } from "../stores/constantsStore";
import { useAssetsStore } from "../stores/assetsStore";
import { useSettingsStore } from "../stores/settingsStore";
import { useLineupsStore, lineupSlug, lineupUrl, LINEUP_MAPS, type LineupFilters } from "../stores/lineupsStore";

export function LineupsPage() {
  const { t } = useI18n();
  const { agent, map, side, setFilters } = useLineupsStore();
  const constants = useConstantsStore(s => s.constants);
  const assets = useAssetsStore(s => s.agents);
  const visible = useSettingsStore(s => s.isWindowVisible);
  const [applied, setApplied] = useState<LineupFilters | null>(null);
  const [revision, setRevision] = useState(0);
  const [loaded, setLoaded] = useState(false);
  const [slow, setSlow] = useState(false);
  const [focused, setFocused] = useState(false);
  const [error, setError] = useState(false);
  const agents = useMemo(() => {
    const names = new Map<string, string>();
    for (const name of [...AGENTS, ...(constants?.agents.map(a => a.name) ?? [])]) {
      names.set(lineupSlug(name), name === "kayo" ? "KAY/O" : name.charAt(0).toUpperCase() + name.slice(1));
    }
    for (const value of assets.values()) names.set(lineupSlug(value.displayName), value.displayName);
    if (!names.has(agent)) names.set(agent, agent.charAt(0).toUpperCase() + agent.slice(1));
    return [...names].sort((a, b) => a[1].localeCompare(b[1]));
  }, [constants, assets, agent]);
  const maps = useMemo(() => {
    const values = LINEUP_MAPS.map(name => [lineupSlug(name), name]);
    if (!values.some(([slug]) => slug === map)) values.push([map, map]);
    return values;
  }, [map]);
  const url = applied ? lineupUrl(applied) : null;
  const dirty = applied && (agent !== applied.agent || map !== applied.map || side !== applied.side);
  const currentUrl = lineupUrl({ agent, map, side });
  const agentName = (slug: string) => agents.find(([id]) => id === slug)?.[1] ?? slug;
  const mapName = (slug: string) => maps.find(([id]) => id === slug)?.[1] ?? slug;

  useEffect(() => {
    if (!url || !visible) return;
    setLoaded(false); setSlow(false); setError(false);
    const timer = setTimeout(() => setSlow(true), 15000);
    return () => clearTimeout(timer);
  }, [url, revision, visible]);
  useEffect(() => {
    if (!focused) return;
    const onKey = (event: KeyboardEvent) => { if (event.key === "Escape") setFocused(false); };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [focused]);
  function show() {
    setLoaded(false); setSlow(false); setError(false);
    setApplied({ agent, map, side }); setRevision(n => n + 1);
  }
  async function openSource() {
    const target = url ?? currentUrl;
    if (!target) return;
    try { await openUrl(target); } catch { setError(true); }
  }

  return (
    <section className={`lineups-page ${focused ? "lineups-focus" : ""}`} aria-label={t("lineups.title")}>
      <div className="lineups-heading"><div><span className="eyebrow">{t("lineups.community")}</span><h2>{t("lineups.title")}</h2><p>{t("lineups.description")}</p></div><button className="lineup-icon" aria-label={focused ? t("lineups.collapse") : t("lineups.expand")} title={focused ? t("lineups.collapse") : t("lineups.expand")} onClick={() => setFocused(v => !v)}>{focused ? <Minimize2 size={17}/> : <Maximize2 size={17}/>}</button></div>
      <form className="lineups-filters" onSubmit={e => { e.preventDefault(); show(); }}>
        <label>{t("lineups.agent")}<select aria-label={t("lineups.agent")} value={agent} onChange={e => setFilters({ agent: e.target.value })}>{agents.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></label>
        <label>{t("lineups.map")}<select aria-label={t("lineups.map")} value={map} onChange={e => setFilters({ map: e.target.value })}>{maps.map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></label>
        <label>{t("lineups.side")}<select aria-label={t("lineups.side")} value={side} onChange={e => setFilters({ side: e.target.value === "defense" ? "defense" : "attack" })}><option value="attack">{t("lineups.attack")}</option><option value="defense">{t("lineups.defense")}</option></select></label>
        <button className="lineups-submit" type="submit" disabled={!currentUrl}><Play size={14}/>{t("lineups.show")}</button>
      </form>
      {dirty && <p className="lineups-notice" role="status">{t("lineups.changed")}</p>}
      <div className="lineups-frame-shell">
        {url && visible ? <>
          {!loaded && !slow && !error && <div className="lineups-loading" role="status"><LoaderCircle size={20} className="animate-spin"/>{t("lineups.loading")}</div>}
          <iframe key={`${url}-${revision}`} src={url} title={t("lineups.frameTitle")} sandbox="allow-scripts allow-same-origin allow-presentation" allow="fullscreen; encrypted-media; picture-in-picture" allowFullScreen referrerPolicy="strict-origin-when-cross-origin" onLoad={() => setLoaded(true)} onError={() => { setLoaded(true); setError(true); }}/>
        </> : <div className="lineups-empty"><span><BookOpen size={30} strokeWidth={1.4}/></span><h3>{t("lineups.chooseTitle")}</h3><p>{t("lineups.chooseDescription")}</p><small>{t("lineups.credit")}</small></div>}
      </div>
      <div className="lineups-source"><span>{applied ? `${agentName(applied.agent)} · ${mapName(applied.map)} · ${t(applied.side === "attack" ? "lineups.attack" : "lineups.defense")}` : t("lineups.credit")}</span><div>{url && <button onClick={() => setRevision(n => n + 1)} title={t("lineups.reload")} aria-label={t("lineups.reload")}><RefreshCw size={13}/></button>}<button onClick={() => void openSource()}><ExternalLink size={12}/>Tracker.gg</button></div></div>
      {(error || (slow && !loaded)) && <p className="lineups-notice" role="status">{t("lineups.unavailable")}</p>}
    </section>
  );
}
