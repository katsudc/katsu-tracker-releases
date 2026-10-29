import type { TrackerMatch } from "../tracker/models";
export type SessionRange = "today" | "yesterday" | "week" | "current" | "custom";
export const localDate = (date: Date) => date.getFullYear() + "-" + String(date.getMonth() + 1).padStart(2, "0") + "-" + String(date.getDate()).padStart(2, "0");
export function dateValue(value: string): Date | null {
  if (!/^\d{4}-\d{2}-\d{2}$/.test(value)) return null;
  const [y, m, d] = value.split("-").map(Number);
  const date = new Date(y, m - 1, d);
  return localDate(date) === value ? date : null;
}
export function sessionBounds(range: SessionRange, now: number, start: number | null, from: string, to: string): [number, number] | null {
  const today = new Date(now); today.setHours(0, 0, 0, 0);
  const end = new Date(today); end.setDate(end.getDate() + 1);
  if (range === "today") return [today.getTime(), end.getTime()];
  if (range === "yesterday") { const day = new Date(today); day.setDate(day.getDate() - 1); return [day.getTime(), today.getTime()]; }
  if (range === "week") { const day = new Date(today); day.setDate(day.getDate() - 6); return [day.getTime(), end.getTime()]; }
  if (range === "current") return [start ?? now, now + 1];
  const a = dateValue(from), b = dateValue(to);
  if (!a || !b || a > b) return null;
  b.setDate(b.getDate() + 1);
  return [a.getTime(), b.getTime()];
}
export function sessionMatches(matches: TrackerMatch[], bounds: [number, number] | null) {
  return !bounds ? [] : matches.filter(m => m.started_at !== null && Number.isFinite(m.started_at) && m.started_at >= bounds[0] && m.started_at < bounds[1]);
}
