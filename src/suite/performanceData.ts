import { acs, aggregate, groups, outcome, type TrackerMatch } from "../tracker/models";
export interface Insight { key: string; values: Record<string, string | number> }
const finite = (v: unknown): v is number => typeof v === "number" && Number.isFinite(v);
export function openingStats(matches: TrackerMatch[]) {
  const eligible = matches.filter(m => finite(m.first_kills) && finite(m.first_deaths) && finite(m.opening_rounds) && m.opening_rounds! > 0);
  if (!eligible.length) return null;
  const kills = eligible.reduce((n, m) => n + m.first_kills!, 0), deaths = eligible.reduce((n, m) => n + m.first_deaths!, 0), rounds = eligible.reduce((n, m) => n + m.opening_rounds!, 0);
  return { kills, deaths, rounds, matches: eligible.length, ratio: deaths > 0 ? kills / deaths : null, deathRate: deaths / rounds * 100 };
}
export function performanceInsights(matches: TrackerMatch[]): Insight[] {
  const rows = [...matches].sort((a, b) => (b.started_at ?? 0) - (a.started_at ?? 0)), insights: Insight[] = [];
  const all = aggregate(rows);
  if (rows.length >= 10 && rows.every(m => acs(m.me) !== null)) {
    const recent = aggregate(rows.slice(0, 5)).acs;
    if (recent !== null && all.acs !== null && all.acs > 0) insights.push({ key: "suite.insightACS", values: { recent: recent.toFixed(0), average: all.acs.toFixed(0), change: ((recent / all.acs - 1) * 100).toFixed(0), n: rows.length } });
  }
  for (const kind of ["agent", "map"] as const) {
    const eligible = groups(rows, kind).filter(g => g.matches >= 5 && g.winrate !== null).sort((a, b) => b.winrate! - a.winrate!);
    const best = eligible[0];
    if (best && all.winrate !== null && best.winrate! >= all.winrate + 5) insights.push({ key: "suite.insightGroup", values: { name: best.name, wr: best.winrate!.toFixed(0), n: best.matches, overall: all.winrate.toFixed(0) } });
  }
  const recent = openingStats(rows.slice(0, 5)), previous = openingStats(rows.slice(5));
  if (recent && previous && recent.matches >= 5 && previous.matches >= 5) insights.push({ key: "suite.insightFD", values: { recent: recent.deathRate.toFixed(1), previous: previous.deathRate.toFixed(1), n: recent.rounds + previous.rounds } });
  return insights;
}
export function knownRR(matches: TrackerMatch[]): number | null {
  return matches.length && matches.every(m => finite(m.rr_delta)) ? matches.reduce((n, m) => n + m.rr_delta!, 0) : null;
}
export function lossStreak(matches: TrackerMatch[]): number {
  let streak = 0;
  for (const m of [...matches].sort((a, b) => (b.started_at ?? 0) - (a.started_at ?? 0))) { if (outcome(m) !== "loss") break; streak++; }
  return streak;
}
export function recapData(matches: TrackerMatch[]) {
  const rows = [...matches].sort((a, b) => (a.started_at ?? 0) - (b.started_at ?? 0));
  const agents = groups(rows, "agent"), maps = groups(rows, "map");
  const ranked = rows.map(m => m.tier_after ?? m.me.tier).filter(finite);
  return { rows, stats: aggregate(rows), firstRank: rows.find(m => finite(m.me.tier))?.me.tier ?? null, endRank: [...rows].reverse().map(m=>m.tier_after??m.me.tier).find(finite)??null, peak: ranked.length ? Math.max(...ranked) : null,
    mostAgent: agents[0] ?? null, mostMap: maps[0] ?? null,
    bestAgent: agents.filter(v => v.matches >= 5 && v.winrate !== null).sort((a, b) => b.winrate! - a.winrate!)[0] ?? null,
    bestMap: maps.filter(v => v.matches >= 5 && v.winrate !== null).sort((a, b) => b.winrate! - a.winrate!)[0] ?? null,
    biggestRR: rows.filter(m => finite(m.rr_delta)).sort((a, b) => b.rr_delta! - a.rr_delta!)[0]?.rr_delta ?? null };
}
