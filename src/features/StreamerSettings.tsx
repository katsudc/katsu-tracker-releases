import { EyeOff } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { usePersonalStore } from "./personalStore";
export function StreamerSettings() {
  const { t } = useI18n();
  const { stream, setStream } = usePersonalStore();
  return <section className="personal-card">
    <div className="personal-heading"><h3><EyeOff size={17}/>{t("stream.title")}</h3><button className="personal-switch" role="switch" aria-label={t("stream.title")} aria-checked={stream.enabled} onClick={() => setStream({ enabled: !stream.enabled })}>{t(stream.enabled ? "spike.on" : "spike.off")}</button></div>
    <p>{t("stream.hint")}</p>
    <div className="stream-preferences">{(["hideNames", "quietNotifications", "hideMusic"] as const).map((key, i) => <label className="personal-check" key={key}><input type="checkbox" checked={stream[key]} onChange={e => setStream({ [key]: e.target.checked })}/>{t(["stream.names", "stream.notifications", "stream.music"][i])}</label>)}</div>
    <small className="personal-muted">{t("stream.discord")}</small>
  </section>;
}
