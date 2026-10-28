import { memo, useEffect, useRef, useState } from "react";
import { useShallow } from "zustand/react/shallow";
import { usePanelStore, type HoveredAgent } from "../stores/panelStore";
import { useI18n } from "../lib/i18n";
import { loadAgentPreviewImage } from "./agentPreviewImages";
import "./agent-picker.css";

interface Preview {
  agent: HoveredAgent;
  portrait: string | null;
  splash: string | null;
  loading: boolean;
}

export const AgentOverlay = memo(function AgentOverlay() {
  const { hoveredAgent, isOpen, panelType } = usePanelStore(useShallow(s => ({ hoveredAgent:s.hoveredAgent, isOpen:s.isOpen, panelType:s.panelType })));
  const { t } = useI18n();
  const [preview, setPreview] = useState<Preview | null>(null);
  const current = useRef<Preview | null>(null);
  current.current = preview;
  const active = isOpen && panelType === "settings";

  useEffect(() => {
    if (!active) { setPreview(null); return; }
    if (!hoveredAgent) {
      const timer = setTimeout(() => setPreview(null), 160);
      return () => clearTimeout(timer);
    }
    let disposed = false;
    const agent = hoveredAgent;
    const portrait = agent.mapOnly ? null : agent.bustPortrait || agent.displayIcon || null;
    const splash = agent.mapContext?.mapSplash || null;
    if (!current.current) setPreview({ agent, portrait:null, splash:null, loading:true });
    const loadPortrait = async () => {
      if (!portrait) return null;
      if (await loadAgentPreviewImage(portrait)) return portrait;
      if (agent.displayIcon && agent.displayIcon !== portrait && await loadAgentPreviewImage(agent.displayIcon)) return agent.displayIcon;
      return null;
    };
    void Promise.all([
      loadPortrait(),
      splash ? loadAgentPreviewImage(splash).then(ok => ok ? splash : null) : Promise.resolve(null),
    ]).then(([portraitUrl, splashUrl]) => {
      if (!disposed) setPreview({ agent, portrait:portraitUrl, splash:splashUrl, loading:false });
    });
    return () => { disposed = true; };
  }, [hoveredAgent, active]);

  if (!active || !preview) return null;
  const { agent, portrait, splash, loading } = preview;
  const color = agent.mapContext?.mapColor || "var(--color-accent-cyan)";
  const mapOnly = agent.mapOnly === true;
  return <div className="agent-preview" data-agent-preview data-agent-name={agent.name}>
    {splash && <img key={splash} src={splash} alt="" decoding="async" draggable={false} className={`agent-preview-map ${mapOnly ? "is-map-only" : ""}`}/>}
    <div className="agent-preview-shade"/>
    <div className="agent-preview-frame" style={{ borderColor:color }}/>
    <div className={`agent-preview-content ${mapOnly ? "is-map-only" : ""}`}>
      {!mapOnly && agent.mapContext && <span className="agent-preview-map-name" style={{ color }}>{agent.mapContext.mapName}</span>}
      <span className="agent-preview-label">{t(mapOnly ? "lineups.map" : agent.mapContext ? "settings.mapPrefs" : "settings.defaultAgent")}</span>
      {!mapOnly && <div className="agent-preview-art">
        {portrait ? <img key={portrait} src={portrait} alt="" decoding="async" draggable={false} className="agent-preview-portrait"/> : loading ? <span className="agent-preview-spinner"/> : <span className="agent-preview-fallback">{agent.name[0]}</span>}
      </div>}
      {mapOnly && loading && <span className="agent-preview-spinner"/>}
      <h2>{agent.name}</h2>
      <i className="agent-preview-rule" style={{ background:color }}/>
    </div>
  </div>;
});
