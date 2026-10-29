import { useEffect, useRef, useState } from "react";
import { useI18n } from "../lib/i18n";
import { useTrackerStore } from "../tracker/store";
import { aggregate } from "../tracker/models";
import { usePersonalStore } from "../features/personalStore";
import { useGameStore } from "../stores/gameStore";
import { toast } from "../features/notifications";
import { localDate, sessionBounds, sessionMatches } from "../features/sessionData";
import { EMPTY_GOAL, useGoalStore, type SessionGoal } from "./goalStore";
import { knownRR, lossStreak } from "./performanceData";
import { accountId } from "./storage";

export function SessionGoals() {
  const { t } = useI18n(), data = useTrackerStore(s => s.data), account = data.profile?.puuid;
  const saved = useGoalStore(s => account ? s.byAccount[account] : undefined), save = useGoalStore(s => s.save);
  const start = usePersonalStore(s => account ? s.sessionStarts[account] : null);
  const [goal, setGoal] = useState<SessionGoal>(saved ?? EMPTY_GOAL), [message, setMessage] = useState("");
  useEffect(() => { setGoal(saved ?? EMPTY_GOAL); setMessage(""); }, [account, saved]);
  const now = Date.now(), rows = sessionMatches(data.matches, sessionBounds(start ? "current" : "today", now, start ?? null, localDate(new Date()), localDate(new Date())));
  const stats = aggregate(rows), rr = knownRR(rows);
  const fields: Array<[keyof Omit<SessionGoal, "notify">, string, number]> = [["rr", "suite.rrTarget", 1000], ["matches", "suite.maxMatches", 100], ["minutes", "suite.maxMinutes", 1440], ["losses", "suite.lossReminder", 20]];
  return <section className="suite-goals"><div className="personal-heading"><div><h3>{t("suite.goals")}</h3><p>{t("suite.goalsHint")}</p></div><span className="personal-muted">{stats.wins}W · {stats.losses}L · {rows.length}{saved?.matches ? " / " + saved.matches : ""} {t("tracker.matches")}{rr !== null ? " · " + (rr > 0 ? "+" : "") + rr + " RR" : ""}</span></div>
    <form className="personal-form" onSubmit={e => { e.preventDefault(); setMessage(account && save(account, goal) ? t("tools.saved") : t("suite.invalidInput")); }}>
      <div className="suite-goal-fields">{fields.map(([key, label, max]) => <label key={key}>{t(label)}<input aria-label={t(label)} disabled={!accountId(account)} type="number" min={1} max={max} step={1} value={goal[key] ?? ""} onChange={e => setGoal({ ...goal, [key]: e.target.value === "" ? null : Number(e.target.value) })}/></label>)}</div>
      <label className="personal-check"><input type="checkbox" checked={goal.notify} onChange={e => setGoal({ ...goal, notify: e.target.checked })}/>{t("suite.goalNotifications")}</label>
      <div className="personal-actions"><button type="submit" disabled={!accountId(account)}>{t("tools.save")}</button><span role="status" className="personal-muted">{message}</span></div>
    </form>
    {saved?.rr && <>{rr!==null&&<div className="suite-goal-progress" role="progressbar" aria-label={t("suite.rrTarget")} aria-valuenow={Math.max(0, rr ?? 0)} aria-valuemin={0} aria-valuemax={saved.rr}><i style={{ width: Math.max(0, Math.min(100, (rr ?? 0) / saved.rr * 100)) + "%" }}/></div>}<p className="personal-muted">{rr === null ? t("suite.rrMissing") : rr + " / " + saved.rr + " RR"}</p></>}
    {!accountId(account) && <p className="personal-muted">{t("suite.noAccount")}</p>}
    {!start && saved?.minutes && <p className="personal-muted">{t("suite.sessionRequired")}</p>}
  </section>;
}
export function GoalReminders() {
  const data = useTrackerStore(s => s.data), game = useGameStore(s => s.gameState.state), account = data.profile?.puuid;
  const goal = useGoalStore(s => account ? s.byAccount[account] : undefined), start = usePersonalStore(s => account ? s.sessionStarts[account] : null);
  const seen = useRef(new Set<string>()), pending = useRef<string|null>(null);
  useEffect(()=>{pending.current=null;},[account,start]);
  useEffect(() => {
    if (!goal?.notify || !account || !start) return;
    const rows = sessionMatches(data.matches, [start, Date.now() + 1]), rr = knownRR(rows);
    const reasons: string[] = [];
    const t = useI18n.getState().t;
    if (goal.matches && rows.length >= goal.matches) reasons.push(t("suite.matchLimit"));
    if (goal.losses && lossStreak(rows) >= goal.losses) reasons.push(t("suite.lossCount", { n: goal.losses }));
    if (goal.rr && rr !== null && rr >= goal.rr) reasons.push(t("suite.rrLimit"));
    if (pending.current === account+":"+start && game === "idle") { pending.current = null; reasons.push(t("suite.timeLimit")); }
    if (game !== "idle") return;
    reasons.forEach(reason => { const id = account + ":" + start + ":" + reason; if (!seen.current.has(id)) { if (seen.current.size > 100) seen.current.clear(); seen.current.add(id); toast.success(t("suite.goalReminder", { reason })); } });
  }, [goal, account, start, data.matches, game]);
  useEffect(() => {
    if (!goal?.notify || !goal.minutes || !start || !account) return;
    const target = start + goal.minutes * 60000, key = account + ":" + start + ":duration";
    if (seen.current.has(key)) return;
    const id = setTimeout(() => {
      seen.current.add(key);
      if (useGameStore.getState().gameState.state === "idle") toast.success(useI18n.getState().t("suite.goalReminder", { reason: useI18n.getState().t("suite.timeLimit") }));
      else pending.current = account+":"+start;
    }, Math.max(0, target - Date.now()));
    return () => clearTimeout(id);
  }, [goal, start, account]);
  return null;
}
