import { useI18n } from "../lib/i18n";
import { SettingsTransfer } from "./SettingsTransfer";
import { StreamerSettings } from "./StreamerSettings";
import { useSuiteNavigation } from "../suite/navigation";
export function ToolsPage({ onSetup, onSpike }: { onSetup: () => void; onSpike: () => void }) {
  const { t } = useI18n();
  return <section className="personal-tools-page">
    <div className="personal-tools-scroll">
      <section className="personal-card setup-invite"><div><span className="eyebrow">{t("tools.setup")}</span><h3>{t("setup.title")}</h3><p>{t("setup.description")}</p></div><button onClick={onSetup}>{t("tools.restartSetup")}</button></section>
      <SettingsTransfer/><StreamerSettings/><div className="personal-actions"><button onClick={()=>useSuiteNavigation.getState().setTool("diagnostics")}>{t("diag.title")}</button><button onClick={onSpike}>{t("spike.title")}</button></div>
    </div>
  </section>;
}
