import { create } from "zustand";

interface AgentAsset {
  displayName: string;
  displayIcon: string;
  displayIconSmall: string;
  bustPortrait: string | null;
}

interface MapAsset {
  displayName: string;
  splash: string;
  displayIcon: string;
  listViewIcon: string;
}

interface AssetsStore {
  agents: Map<string, AgentAsset>;
  maps: Map<string, MapAsset>;
  loaded: boolean;
  loadAssets: () => Promise<void>;
  getAgentIcon: (agentName: string) => string | null;
  getAgentAsset: (agentName: string) => AgentAsset | null;
  getMapSplash: (mapName: string) => string | null;
  getMapThumbnail: (mapName: string) => string | null;
}

export const useAssetsStore = create<AssetsStore>((set, get) => ({
  agents: new Map(),
  maps: new Map(),
  loaded: false,

  loadAssets: async () => {
    // Allow retry if a previous attempt marked loaded but produced no data
    if (get().loaded && get().agents.size > 0) return;

    try {
      const [agentsRes, mapsRes] = await Promise.all([
        fetch("https://valorant-api.com/v1/agents?isPlayableCharacter=true"),
        fetch("https://valorant-api.com/v1/maps"),
      ]);

      if (!agentsRes.ok || !mapsRes.ok) {
        throw new Error(`valorant-api HTTP ${agentsRes.status}/${mapsRes.status}`);
      }

      const agentsData = await agentsRes.json();
      const mapsData = await mapsRes.json();

      const agentsMap = new Map<string, AgentAsset>();
      const mapsMap = new Map<string, MapAsset>();

      for (const agent of agentsData.data || []) {
        const name = agent.displayName?.replace(/\//g, "").toLowerCase();
        if (name) {
          agentsMap.set(name, {
            displayName: agent.displayName,
            displayIcon: agent.displayIcon || "",
            displayIconSmall: agent.displayIconSmall || agent.displayIcon || "",
            bustPortrait: agent.bustPortrait || null,
          });
        }
      }

      for (const map of mapsData.data || []) {
        const name = map.displayName;
        if (name) {
          mapsMap.set(name.toLowerCase(), {
            displayName: name,
            splash: map.splash || "",
            displayIcon: map.displayIcon || "",
            listViewIcon: map.listViewIcon || "",
          });
        }
      }

      if (agentsMap.size === 0) {
        throw new Error("valorant-api returned 0 agents");
      }

      set({ agents: agentsMap, maps: mapsMap, loaded: true });
      // Full-size map art is decoded only when a visible view requests it.
    } catch (error) {
      console.error("Failed to load assets:", error);
      // Do NOT set loaded:true on failure — allows a later retry
      set({ loaded: false });
    }
  },

  getAgentIcon: (agentName: string) => {
    const agent = get().agents.get(agentName.toLowerCase());
    const icon = agent?.displayIconSmall || agent?.displayIcon || "";
    return icon || null;
  },

  getAgentAsset: (agentName: string) => {
    return get().agents.get(agentName.toLowerCase()) || null;
  },

  getMapThumbnail: (mapName: string) => {
    const map=get().maps.get(mapName.toLowerCase());
    return map?.listViewIcon || map?.splash || null;
  },

  getMapSplash: (mapName: string) => {
    const map = get().maps.get(mapName.toLowerCase());
    return map?.splash || map?.listViewIcon || null;
  },

}));
