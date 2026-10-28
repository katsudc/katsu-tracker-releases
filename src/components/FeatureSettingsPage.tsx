import { LockKeyhole, Timer } from "lucide-react";
import { SettingsPanel } from "./SettingsPanel";
import { SpikeSettings } from "./SpikeTimer";
import { useI18n } from "../lib/i18n";

export function FeatureSettingsPage({ kind }: { kind: "instalock" | "spike" }) {
  const { t } = useI18n();
  const Icon = kind === "instalock" ? LockKeyhole : Timer;
  return <section className="feature-settings-page" aria-label={t(`${kind}.title`)}>
    <header className="feature-page-heading">
      <span className="feature-page-icon"><Icon size={22}/></span>
      <div><h2>{t(`${kind}.title`)}</h2><p>{t(`${kind}.pageDescription`)}</p></div>
    </header>
    <div className={`feature-settings-content feature-${kind}`}>
      {kind === "instalock" ? <SettingsPanel section="instalock"/> : <SpikeSettings/>}
    </div>
  </section>;
}
