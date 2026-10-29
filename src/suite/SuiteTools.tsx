import { lazy, Suspense } from "react";
import { Wrench } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useSuiteNavigation, type ToolTab } from "./navigation";
import { FeatureBoundary } from "./FeatureBoundary";
const Home=lazy(()=>import("../features/ToolsPage").then(m=>({default:m.ToolsPage})));
const Lineups=lazy(()=>import("../components/LineupsPage").then(m=>({default:m.LineupsPage})));
const Profiles=lazy(()=>import("./ProfileTools").then(m=>({default:m.ProfileTools})));
const Notes=lazy(()=>import("./NotesTools").then(m=>({default:m.NotesTools})));
const Diagnostics=lazy(()=>import("./SuiteDiagnostics").then(m=>({default:m.SuiteDiagnostics})));
const Privacy=lazy(()=>import("./PrivacyDashboard").then(m=>({default:m.PrivacyDashboard})));
export function SuiteTools({onSetup,onSpike}:{onSetup:()=>void;onSpike:()=>void}){const {t}=useI18n(),tab=useSuiteNavigation(s=>s.tool),setTab=useSuiteNavigation(s=>s.setTool);const tabs:Array<[ToolTab,string]>=[["home","suite.overview"],["lineups","lineups.title"],["crosshairs","suite.crosshairs"],["sensitivity","suite.sensitivity"],["notes","suite.notes"],["diagnostics","diag.title"],["privacy","suite.privacy"]];return <section className="suite-tools"><header className="feature-page-heading"><span className="feature-page-icon"><Wrench size={23}/></span><div><h2>{t("tools.title")}</h2><p>{t("tools.description")}</p></div></header><nav className="personal-tabs suite-subnav" aria-label={t("tools.title")}>{tabs.map(([id,key])=><button key={id} aria-pressed={tab===id} onClick={()=>setTab(id)}>{t(key)}</button>)}</nav><div className="suite-tool-scroll"><FeatureBoundary key={tab}><Suspense fallback={<p className="personal-empty">{t("suite.loading")}</p>}>{tab==="home"?<Home onSetup={onSetup} onSpike={onSpike}/>:tab==="lineups"?<Lineups libraryOnly/>:tab==="crosshairs"||tab==="sensitivity"?<Profiles key={tab} kind={tab}/>:tab==="notes"?<Notes/>:tab==="diagnostics"?<Diagnostics onSpike={onSpike}/>:<Privacy/>}</Suspense></FeatureBoundary></div></section>;}
