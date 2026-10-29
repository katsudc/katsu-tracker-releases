import "./desktop.css";
import "./features/personal.css";
import "./suite/suite.css";
import { RuntimeFeatures, useTrayBridge, useMomentShortcut } from "./suite/RuntimeFeatures";
import { useRuntimeStore } from "./suite/runtimeStore";
import { useSuiteNavigation } from "./suite/navigation";
import { CommandPalette } from "./suite/CommandPalette";
import { GoalReminders } from "./suite/SessionGoals";
import { useAccountLifecycle } from "./suite/AccountProfiles";
import { FeatureBoundary } from "./suite/FeatureBoundary";
import { SetupWizard } from "./features/SetupWizard";
import { usePersonalStore } from "./features/personalStore";
import { useChatStore } from "./stores/chatStore";
import { invoke } from "./suite/ipc";
import { UpdateNotice } from "./components/UpdateNotice";
import { GameBarSetupNotice } from "./components/SpikeOutputSettings";
import "./tracker/tracker.css";
import { useTrackerLifecycle } from "./tracker/lifecycle";


import { DesktopSidebar } from "./components/DesktopSidebar";
import { FeatureSettingsPage } from "./components/FeatureSettingsPage";
import { DashboardOverview } from "./components/DashboardOverview";
import { Header } from "./components/Header";
import { Footer } from "./components/Footer";
import { PregameState } from "./components/PregameState";
import { IngameState } from "./components/IngameState";
import { SidePanel } from "./components/SidePanel";
import { WeaponOverlay } from "./components/WeaponOverlay";
import { AgentOverlay } from "./components/AgentOverlay";
import { CrosshairOverlay } from "./components/CrosshairOverlay";
import { ChatPanel } from "./components/Chat/ChatPanel";
import { useGameStore } from "./stores/gameStore";
import { useSettingsStore } from "./stores/settingsStore";
import { usePanelStore } from "./stores/panelStore";
import { useGameLoop } from "./hooks/useGameLoop";
import { useI18n } from "./lib/i18n";
import { Toaster } from "sonner";
import { toast } from "./features/notifications";
import { lazy, Suspense, useEffect, useState } from "react";
import { RefreshCw, X } from "lucide-react";

const SuiteTools=lazy(()=>import("./suite/SuiteTools").then(m=>({default:m.SuiteTools})));
const PerformancePage=lazy(()=>import("./suite/PerformanceViews").then(m=>({default:m.PerformancePage})));
const LineupsPage=lazy(()=>import("./components/LineupsPage").then(m=>({default:m.LineupsPage})));

/** How long the first-launch tip stays under the header (ms). */
const WELCOME_TIP_DURATION_MS = 6_500;

/** WebView / browser chrome shortcuts that should never surface in the overlay. */
function isBrowserChromeShortcut(e: KeyboardEvent): boolean {
  if (e.key === "F3" || e.key === "F5") return true;

  if (!(e.ctrlKey || e.metaKey) || e.altKey) return false;

  switch (e.key.toLowerCase()) {
    case "f": // find
    case "g": // find next
    case "p": // print
    case "s": // save
    case "u": // view source
    case "h": // history
    case "j": // downloads
    case "r": // reload
    case "+":
    case "-":
    case "=":
    case "_":
    case "0":
      return true;
    default:
      return false;
  }
}

/** Guard StrictMode double-mount so the tip only schedules once per session. */
let welcomeTipScheduled = false;

function WelcomeTipBanner({
  hotkey,
  onDismiss,
}: {
  hotkey: string;
  onDismiss: () => void;
}) {
  const { t } = useI18n();

  return (
    <div
      className="mb-2 shrink-0 rounded-lg border border-accent-cyan/20 bg-accent-cyan/[0.07] px-2.5 py-1.5 animate-smooth-appear"
      role="status"
    >
      <div className="flex items-start gap-2">
        <div className="min-w-0 flex-1">
          <div className="text-[9px] font-black uppercase tracking-[0.14em] text-accent-cyan/90 mb-0.5">
            {t("welcome.title")}
          </div>
          <p className="text-[10px] leading-snug text-primary/85">
            {t("welcome.hotkey", { hotkey })}
          </p>
          <p className="text-[9px] leading-snug text-dim/90 mt-0.5">
            {t("welcome.borderless")}
          </p>
        </div>
        <button
          type="button"
          onClick={onDismiss}
          className="shrink-0 w-5 h-5 flex items-center justify-center rounded text-dim/70 hover:text-primary hover:bg-white/10 transition-colors"
          aria-label={t("common.close")}
        >
          <X className="w-3 h-3" strokeWidth={2.5} />
        </button>
      </div>
    </div>
  );
}

function App() {
  useTrackerLifecycle();
  useTrayBridge(); useMomentShortcut(); useAccountLifecycle();
  const runtime = useRuntimeStore(s=>s.mode);
  useEffect(()=>{document.documentElement.dataset.safeMode=runtime==="normal"?"false":"true";},[runtime]);
  useEffect(()=>{try{localStorage.removeItem("katsu-experimental-scouting-v1");}catch{/* Legacy preferences are optional. */}},[]);
  const { gameState, status, pendingReconnect } = useGameStore();
  const { hotkey, theme } = useSettingsStore();
  const view=useSuiteNavigation(s=>s.view);
  const setView=(value:string)=>useSuiteNavigation.getState().go(value);
  const [setupOpen, setSetupOpen] = useState(false);
  const [setupManual, setSetupManual] = useState(false);
  const stream = usePersonalStore(s => s.stream);
  const setupComplete = usePersonalStore(s => s.setupComplete);
  const discordRpc = useSettingsStore(s => s.discordRpcEnabled);
  const privateNames = stream.enabled && stream.hideNames;
  const quiet = stream.enabled && stream.quietNotifications;
  useEffect(() => {
    if (setupComplete || setupManual || runtime!=="normal") return;
    if (gameState.state !== "idle") { setSetupOpen(false); return; }
    const timer = setTimeout(() => { if (useGameStore.getState().gameState.state === "idle") setSetupOpen(true); }, 1200);
    return () => clearTimeout(timer);
  }, [setupComplete, setupManual, gameState.state, runtime]);
  useEffect(() => { if (quiet) toast.dismiss(); }, [quiet]);
  useEffect(() => { if (privateNames) useChatStore.getState().setIsOpen(false); }, [privateNames]);
  useEffect(() => { void invoke("set_discord_rpc", { enabled: !stream.enabled && discordRpc }).catch(() => {}); }, [stream.enabled, discordRpc]);
  const openTools = () => { setView("tools"); void usePanelStore.getState().close(); };
  useEffect(()=>{document.documentElement.dataset.theme=theme;},[theme]);
  const { t } = useI18n();
  const [showWelcomeTip, setShowWelcomeTip] = useState(false);
  const showReconnectBar =
    (pendingReconnect || status === "RECONNECTING") &&
    (gameState.state === "pregame" || gameState.state === "ingame");

  // Subscribe to backend connection/game-state events (backend owns the loop)
  useGameLoop();

  // Disable context menu and WebView chrome shortcuts (find, print, reload, zoom…)
  useEffect(() => {
    const handleContextMenu = (e: MouseEvent) => {
      e.preventDefault();
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (isBrowserChromeShortcut(e)) {
        e.preventDefault();
      }
    };

    const handleWheel = (e: WheelEvent) => {
      if (e.ctrlKey || e.metaKey) {
        e.preventDefault();
      }
    };

    document.addEventListener("contextmenu", handleContextMenu);
    // Capture so we beat focused inputs; native WebView2 Find is also
    // disabled in Rust via AreBrowserAcceleratorKeysEnabled=false.
    window.addEventListener("keydown", handleKeyDown, true);
    window.addEventListener("wheel", handleWheel, { passive: false });

    return () => {
      document.removeEventListener("contextmenu", handleContextMenu);
      window.removeEventListener("keydown", handleKeyDown, true);
      window.removeEventListener("wheel", handleWheel);
    };
  }, []);

  // First launch: compact tip under the VALORANT header (once).
  useEffect(() => {
    if (welcomeTipScheduled) return;
    welcomeTipScheduled = true;

    let hideTimer: number | undefined;
    let showTimer: number | undefined;

    const run = () => {
      const { hasSeenWelcome, markWelcomeSeen } = useSettingsStore.getState();
      if (hasSeenWelcome) return;

      // Short delay so the window is painted first.
      showTimer = window.setTimeout(() => {
        markWelcomeSeen();
        setShowWelcomeTip(true);
        hideTimer = window.setTimeout(() => setShowWelcomeTip(false), WELCOME_TIP_DURATION_MS);
      }, 500);
    };

    if (useSettingsStore.persist.hasHydrated()) {
      run();
    } else {
      const unsub = useSettingsStore.persist.onFinishHydration(run);
      return () => {
        unsub();
        if (showTimer) window.clearTimeout(showTimer);
        if (hideTimer) window.clearTimeout(hideTimer);
      };
    }

    return () => {
      if (showTimer) window.clearTimeout(showTimer);
      if (hideTimer) window.clearTimeout(hideTimer);
    };
  }, []);

  const renderContent = () => {
    switch (gameState.state) {
      case "pregame":
        return <PregameState />;
      case "ingame":
        return <IngameState />;
      default:
        return <DashboardOverview />;
    }
  };

  return (
    <div className="desktop-shell">
      <Header />
      <UpdateNotice />
      <RuntimeFeatures/>
      <CommandPalette/>
      <GoalReminders/>
      <GameBarSetupNotice />
      <div className="desktop-body">
      <DesktopSidebar view={view} setView={setView}/>
      <div className="tracker-workspace">
        {showWelcomeTip && !quiet && !setupOpen && (
          <WelcomeTipBanner
            hotkey={hotkey}
            onDismiss={() => setShowWelcomeTip(false)}
          />
        )}
        {showReconnectBar && (
          <div
            className="mb-2 shrink-0 flex items-center gap-2 rounded-lg border border-accent-gold/25 bg-accent-gold/[0.08] px-2.5 py-1.5 animate-smooth-appear"
            role="status"
          >
            <RefreshCw className="w-3.5 h-3.5 animate-spin text-accent-gold shrink-0" strokeWidth={2.5} />
            <span className="text-[10px] font-bold tracking-wider text-accent-gold uppercase">
              {t("waiting.reconnecting")}
            </span>
          </div>
        )}
        <FeatureBoundary key={view}><Suspense fallback={<p className="personal-empty">{t("suite.loading")}</p>}>{view==="tools"?<SuiteTools onSetup={()=>{setSetupManual(true);setSetupOpen(true);}} onSpike={()=>setView("spike")}/>:view==="instalock"||view==="spike"?<FeatureSettingsPage key={view+runtime} kind={view}/>:view==="lineups"?<LineupsPage/>:view==="statistics"?<PerformancePage/>:renderContent()}</Suspense></FeatureBoundary>
        <Footer />

        {/* Weapon hover overlay */}
        {runtime==="normal" && <WeaponOverlay />}

        {/* Agent hover overlay for settings */}
        {runtime==="normal" && <AgentOverlay key="agent-overlay" enabled={view === "instalock"} />}

        {/* Crosshair hover preview for presets */}
        {runtime==="normal" && <CrosshairOverlay />}
      </div>

      {/* Side panel */}
      <SidePanel onTools={openTools}/>

      </div>
      {/* Chat Panel Overlay */}
      {!privateNames && <ChatPanel />}
      {setupOpen && runtime==="normal" && <SetupWizard onClose={()=>{setSetupOpen(false);setSetupManual(false);}}/>}

      {!quiet && <Toaster position="top-right" richColors theme="dark" />}
    </div>
  );
}

export default App;
