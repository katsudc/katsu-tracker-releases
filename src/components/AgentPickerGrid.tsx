import { memo } from "react";
import { useGameStore } from "../stores/gameStore";
import { useConstantsStore } from "../stores/constantsStore";
import { useAssetsStore } from "../stores/assetsStore";
import { useI18n } from "../lib/i18n";
import { MAP_METADATA, type CompetitiveMap } from "../lib/maps";
import { CachedImage } from "./CachedImage";

interface Props {
  map?: CompetitiveMap;
  onNameHover: (context: string, name: string | null) => void;
  onPreview: (name: string, context?: { mapName: string; mapSplash: string | null; mapColor: string }) => void;
  onPreviewEnd: () => void;
  onMapRestore: (name: string, splash: string | null, color: string) => void;
}

/** Hovering a different portrait must not rebuild all portrait grids. */
export const AgentPickerGrid = memo(function AgentPickerGrid({ map, onNameHover, onPreview, onPreviewEnd, onMapRestore }: Props) {
  const agents = useConstantsStore(s => s.constants?.agents);
  const icons = useAssetsStore(s => s.agents);
  const maps = useAssetsStore(s => s.maps);
  const selected = useGameStore(s => map ? s.mapAgentPreferences[map] : s.autoLockAgent);
  const fallback = useGameStore(s => s.autoLockAgent);
  const select = useGameStore(s => s.setAutoLock);
  const { t } = useI18n();
  const context = map ?? "global";
  const mapAsset = map ? maps.get(map.toLowerCase()) : null;
  const splash = mapAsset?.splash || mapAsset?.listViewIcon || null;
  const color = map ? MAP_METADATA[map]?.color || "#ff8eae" : "#ff8eae";

  function leave() {
    onNameHover(context, null);
    if (map) onMapRestore(map, splash, color);
    else onPreviewEnd();
  }

  return <div className={`agent-picker-grid ${map ? "" : "agent-picker-grid-default"}`} data-agent-grid={context}>
    {(agents ?? []).map(agent => {
      const chosen = selected === agent.uuid;
      const inherited = !!map && !selected && fallback === agent.uuid;
      const asset = icons.get(agent.name.toLowerCase());
      const icon = asset?.displayIconSmall || asset?.displayIcon;
      const title = inherited ? `${agent.name.toUpperCase()} (${t("settings.defaultTag")})` : agent.name.toUpperCase();
      const preview = () => {
        onNameHover(context, agent.name);
        onPreview(agent.name, map ? { mapName: map, mapSplash: splash, mapColor: color } : undefined);
      };
      return <button key={agent.uuid} type="button" title={title} aria-label={title} aria-pressed={chosen}
        className={`agent-picker-choice ${chosen ? "is-selected" : inherited ? "is-inherited" : ""}`}
        onClick={() => select(chosen ? null : agent.uuid, map)} onMouseEnter={preview} onMouseLeave={leave} onFocus={preview} onBlur={leave}>
        {icon ? <CachedImage src={icon} alt={agent.name} reveal={false} className="agent-picker-icon"/> : <span>{agent.name[0]?.toUpperCase()}</span>}
      </button>;
    })}
  </div>;
});
