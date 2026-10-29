import { useEffect, useState } from "react";
import { CalendarDays, RotateCcw } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useTrackerStore } from "../tracker/store";
import { aggregate } from "../tracker/models";
import { usePersonalStore } from "./personalStore";
import { localDate, sessionBounds, sessionMatches, type SessionRange } from "./sessionData";

export function DailySession() {
  const { t, locale } = useI18n();
  const data = useTrackerStore(s => s.data);
  const account = data.profile?.puuid;
  const start = usePersonalStore(s => account ? s.sessionStarts[account] ?? null : null);
  const startSession = usePersonalStore(s => s.startSession);
  const [range, setRange] = useState<SessionRange>("today");
  const [from, setFrom] = useState(localDate(new Date()));
  const [to, setTo] = useState(localDate(new Date()));
  const [now, setNow] = useState(Date.now());
  const [confirm, setConfirm] = useState(false);
  useEffect(() => { const id = setInterval(() => setNow(Date.now()), 30000); return () => clearInterval(id); }, []);
  const bounds = sessionBounds(range, now, start, from, to), rows = sessionMatches(data.matches, bounds), stats = aggregate(rows);
  const duration = rows.every(m => m.duration_ms != null) ? rows.reduce((n, m) => n + (m.duration_ms ?? 0), 0) : null;
  const number = (n: number | null, digits = 0) => n == null ? "—" : n.toFixed(digits);
  return <section className="daily-session" aria-label={t("session.title")}>
    <div className="personal-heading"><div><span className="eyebrow"><CalendarDays size={12}/> {t("session.title")}</span><h3>{t("tracker.record", { matches: stats.matches, wins: stats.wins, losses: stats.losses })}</h3></div>
      <button disabled={!account} className="personal-quiet" onClick={() => setConfirm(true)}><RotateCcw size={13}/>{t("session.reset")}</button></div>
    <div className="personal-tabs">{(["today", "yesterday", "week", "current", "custom"] as const).map(v => <button key={v} aria-pressed={range === v} onClick={() => setRange(v)}>{t("session." + v)}</button>)}</div>
    {range === "custom" && <div className="session-dates"><label>{t("session.from")}<input aria-label={t("session.from")} type="date" value={from} onChange={e => setFrom(e.target.value)}/></label><label>{t("session.to")}<input aria-label={t("session.to")} type="date" value={to} onChange={e => setTo(e.target.value)}/></label></div>}
    {range === "current" && <p className="personal-muted">{start ? t("session.started") + " · " + new Date(start).toLocaleString(locale === "tr" ? "tr-TR" : "en-US") : t("session.reset")}</p>}
    {!bounds && <p className="personal-error" role="alert">{t("session.dateError")}</p>}
    <div className="session-metrics">{[
      [t("session.wins"), String(stats.wins)], [t("session.losses"), String(stats.losses)], [t("session.draws"), String(stats.draws)],
      ["K/D", number(stats.kd, 2)], ["K/D/A", number(stats.kills, 1) + " / " + number(stats.deaths, 1) + " / " + number(stats.assists, 1)],
      [t("session.time"), rows.length && duration !== null ? Math.round(duration / 60000) + " " + t("tracker.minutes") : "—"],
    ].map(([label, value]) => <div key={label}><span>{label}</span><strong>{value}</strong></div>)}</div>
    {!rows.length && <p className="personal-muted">{t("session.empty")}</p>}
    <p className="personal-muted">{t("session.scope", { n: data.matches.length })}</p>
    {data.matches.some(m => m.started_at == null) && <small className="personal-muted">{t("session.noDates")}</small>}
    {confirm && <div className="personal-confirm" role="alert"><span>{t("session.confirm")}</span><button onClick={() => { if (account) startSession(account); setNow(Date.now()); setRange("current"); setConfirm(false); }}>{t("session.reset")}</button><button onClick={() => setConfirm(false)}>{t("tools.cancel")}</button></div>}
  </section>;
}
