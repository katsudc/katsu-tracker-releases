import { create } from "zustand";
import type { PlayerData, CrosshairLayer } from "../lib/types";

type PanelType = "settings" | "player" | "shop" | null;
export type SettingsSubView = "main" | "presets";
export type PlayerSubView = "skins" | "regulars";
export type PlayerSource = "roster" | "friends";


export interface HoveredWeapon {
  name: string;
  icon: string;
  weaponType: string;
  buddy?: {
    name: string;
    icon: string;
  };
  price?: {
    vp: number;
    discountPercent?: number;
    discountedVp?: number;
  };
  tierColor?: string;
  note?: string;
}

export interface HoveredAgent {
  name: string;
  displayIcon: string;
  bustPortrait: string | null;
  mapContext?: {
    mapName: string;
    mapSplash: string | null;
    mapColor: string;
  };
  /** When true, left overlay shows only the map (no agent portrait). */
  mapOnly?: boolean;
}

export interface HoveredCrosshair {
  name: string;
  layer: CrosshairLayer;
}

interface PanelStore {
  isOpen: boolean;
  panelType: PanelType;
  /** Nested view when panelType is "settings". */
  settingsSubView: SettingsSubView;
  /** Nested view when panelType is "player". */
  playerSubView: PlayerSubView;
  /** Who opened the player panel — friends hides skins / back. */
  playerSource: PlayerSource;
  selectedPlayer: PlayerData | null;
  hoveredWeapon: HoveredWeapon | null;
  hoveredAgent: HoveredAgent | null;
  hoveredCrosshair: HoveredCrosshair | null;

  openSettings: () => Promise<void>;
  openPlayer: (player: PlayerData, source?: PlayerSource) => Promise<void>;
  openShop: () => Promise<void>;
  close: () => Promise<void>;
  /** Match window width to panel content: keep a real pane, collapse an empty one. */
  syncWindowToState: () => Promise<void>;
  setSettingsSubView: (view: SettingsSubView) => void;
  setPlayerSubView: (view: PlayerSubView) => void;
  setHoveredWeapon: (weapon: HoveredWeapon | null) => void;
  setHoveredAgent: (agent: HoveredAgent | null) => void;
  setHoveredCrosshair: (crosshair: HoveredCrosshair | null) => void;
}


/** Invalidates in-flight open/close so a hide during resize cannot leave a ghost panel. */
let panelOpId = 0;

const CLOSED_PANEL = {
  isOpen: false,
  panelType: null as PanelType,
  settingsSubView: "main" as SettingsSubView,
  playerSubView: "skins" as PlayerSubView,
  playerSource: "roster" as PlayerSource,
  selectedPlayer: null,
  hoveredWeapon: null,
  hoveredAgent: null,
  hoveredCrosshair: null,
};

function beginPanelOp() {
  panelOpId += 1;
  return panelOpId;
}

function isCurrentPanelOp(op: number) {
  return op === panelOpId;
}

function hasRenderablePanel(state: {
  isOpen: boolean;
  panelType: PanelType;
  selectedPlayer: PlayerData | null;
}): boolean {
  if (!state.isOpen || !state.panelType) return false;
  if (state.panelType === "player") {
    return state.selectedPlayer != null;
  }
  return true;
}

export const usePanelStore = create<PanelStore>((set, get) => ({
  isOpen: false,
  panelType: null,
  settingsSubView: "main",
  playerSubView: "skins",
  playerSource: "roster",
  selectedPlayer: null,
  hoveredWeapon: null,
  hoveredAgent: null,
  hoveredCrosshair: null,

  openSettings: async () => {
    const op = beginPanelOp();
    if (!isCurrentPanelOp(op)) return;
    set({
      isOpen: true,
      panelType: "settings",
      settingsSubView: "main",
      playerSource: "roster",
      selectedPlayer: null,
      hoveredWeapon: null,
      hoveredAgent: null,
    });
  },

  setSettingsSubView: (view) => set({ settingsSubView: view }),
  setPlayerSubView: (view) => set({ playerSubView: view }),

  openShop: async () => {
    const { isOpen, panelType } = get();

    // Toggle: if shop already open, close it
    if (isOpen && panelType === "shop") {
      await get().close();
      return;
    }

    const op = beginPanelOp();
    if (!isCurrentPanelOp(op)) return;
    set({ isOpen: true, panelType: "shop", playerSource: "roster", selectedPlayer: null, hoveredWeapon: null, hoveredAgent: null });
  },

  openPlayer: async (player, source = "roster") => {
    const { isOpen, panelType, selectedPlayer, playerSource } = get();

    // Toggle: if same player from the same origin clicked again, close panel
    if (isOpen && panelType === "player" && selectedPlayer?.puuid === player.puuid && playerSource === source) {
      await get().close();
      return;
    }

    const op = beginPanelOp();
    if (!isCurrentPanelOp(op)) return;
    set({
      isOpen: true,
      panelType: "player",
      playerSubView: source === "friends" ? "regulars" : "skins",
      playerSource: source,
      selectedPlayer: player,
      hoveredWeapon: null,
      hoveredAgent: null,
    });
  },

  close: async () => {
    const op = beginPanelOp();
    set({ ...CLOSED_PANEL });
    // Wait for exit animation or state update to clear
    await new Promise(resolve => setTimeout(resolve, 300));
    if (!isCurrentPanelOp(op)) return;
  },

  syncWindowToState: async () => {
    const state = get();
    if (!hasRenderablePanel(state)) {
      if (state.isOpen) {
        beginPanelOp();
        set({ ...CLOSED_PANEL });
      }
      return;
    }
  },

  setHoveredWeapon: (weapon) => {
    set({ hoveredWeapon: weapon });
  },

  setHoveredAgent: (agent) => {
    const previous = get().hoveredAgent;
    if (previous === agent) return;
    if (previous && agent && previous.name === agent.name && previous.displayIcon === agent.displayIcon && previous.bustPortrait === agent.bustPortrait && previous.mapOnly === agent.mapOnly && previous.mapContext?.mapName === agent.mapContext?.mapName && previous.mapContext?.mapSplash === agent.mapContext?.mapSplash && previous.mapContext?.mapColor === agent.mapContext?.mapColor) return;
    set({ hoveredAgent: agent });
  },

  setHoveredCrosshair: (crosshair) => {
    set({ hoveredCrosshair: crosshair });
  },
}));
