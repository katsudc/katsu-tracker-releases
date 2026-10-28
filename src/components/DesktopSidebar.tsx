import { useShallow } from "zustand/react/shallow";
import { BarChart3, BookOpen, Crosshair, Settings2, ShoppingBag } from "lucide-react";
import { useTrackerStore } from "../tracker/store";
import { usePanelStore } from "../stores/panelStore";
import { useI18n } from "../lib/i18n";

export function DesktopSidebar({view,setView}:{view:string;setView:(view:string)=>void}) {
  const { t, locale, setLocale } = useI18n();
  const panel = usePanelStore(useShallow(s=>({isOpen:s.isOpen,panelType:s.panelType,openSettings:s.openSettings,openShop:s.openShop,close:s.close})));
  const items = [
    { id: "match", label: t("desktop.match"), icon: Crosshair, run: () => {useTrackerStore.getState().select(null);setView("match");void panel.close();} },
    { id: "statistics", label: t("tracker.statistics"), icon: BarChart3, run: () => {useTrackerStore.getState().select(null);setView("statistics");void panel.close();} },
    { id: "lineups", label: t("lineups.title"), icon: BookOpen, run: () => {setView("lineups");void panel.close();} },
    { id: "shop", label: t("shop.title"), icon: ShoppingBag, run: () => {setView("match");void panel.openShop();} },
    { id: "settings", label: t("settings.title"), icon: Settings2, run: () => {setView("match");void panel.openSettings();} },
  ];
  return (
    <nav className="desktop-sidebar" aria-label={t("desktop.navigation")}>
      <span className="sidebar-monogram">K<span>•</span></span>
      <div className="sidebar-links">
        {items.map(({ id, label, icon: Icon, run }) => {
          const active = panel.isOpen && ["shop","settings"].includes(panel.panelType??"") ? panel.panelType === id : id === view;
          return <button key={id} aria-label={label} title={label} aria-current={active ? "page" : undefined} className={active ? "is-active" : ""} onClick={run}><Icon size={19}/><span>{label}</span></button>;
        })}
      </div>
      <button className="sidebar-language" title={t("settings.language")} onClick={() => setLocale(locale === "tr" ? "en" : "tr")}>{locale.toUpperCase()}</button>
      <span className="sidebar-caption">KATSU</span>
    </nav>
  );
}
