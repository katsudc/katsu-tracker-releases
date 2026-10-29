import { useEffect, useMemo, useRef, useState } from "react";
import { BookMarked, ExternalLink, Pencil, Plus, Search, Star, Trash2 } from "lucide-react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { useI18n } from "../lib/i18n";
import { useLineupsStore } from "../stores/lineupsStore";
import { useLibraryStore } from "./libraryStore";
import { LIBRARY_LIMIT, parseTime, playbackUrl, timeText, videoUrl, type SavedLineup } from "./libraryData";
import { LineupTransfer } from "../suite/LineupTransfer";
import { useSuiteNavigation } from "../suite/navigation";
import { useGameStore } from "../stores/gameStore";
import { lineupSlug } from "../stores/lineupsStore";
import { Dialog } from "./Dialog";

export function LineupLibrary({ agents, maps }: { agents: string[][]; maps: string[][] }) {
  const { t } = useI18n();
  const { agent, map, side } = useLineupsStore();
  const { items, save, remove, favorite } = useLibraryStore();
  const game=useGameStore(s=>s.gameState),me=game.allies.find(p=>p.is_me),focus=useSuiteNavigation(s=>s.focus),input=useRef<HTMLInputElement>(null);
  const [follow,setFollow]=useState(false);
  const rawSide=game.side?.toLowerCase();
  const gameSide=rawSide==="saldıran"||rawSide==="saldiran"?"attack":rawSide==="savunan"?"defense":rawSide;
  const context=game.map_name&&me?.agent&&["attack","defense","attacking","defending"].includes(gameSide??"")?{map:lineupSlug(game.map_name),agent:lineupSlug(me.agent),side:gameSide?.startsWith("attack")?"attack":"defense"}:null;
  const contextKey=context?context.map+":"+context.agent+":"+context.side:"";
  useEffect(()=>{if(follow&&context){useLineupsStore.getState().setFilters({map:context.map,agent:context.agent,side:context.side as "attack"|"defense"});setFiltered(true);}},[follow,contextKey]);
  useEffect(()=>{if(focus==="lineup-search"){input.current?.focus();useSuiteNavigation.getState().consumeFocus();}},[focus]);
  const [editing, setEditing] = useState<SavedLineup | "new" | null>(null);
  const [search, setSearch] = useState("");
  const [filtered, setFiltered] = useState(false);
  const [starred, setStarred] = useState(false);
  const [confirm, setConfirm] = useState<string | null>(null);
  const [error, setError] = useState("");
  const rows = useMemo(() => items.filter(item =>
    (!filtered || (item.agent === agent && item.map === map && item.side === side)) &&
    (!starred || item.favorite) && (item.title + " " + item.notes).toLocaleLowerCase().includes(search.toLocaleLowerCase()),
  ).sort((a, b) => Number(b.favorite) - Number(a.favorite) || b.createdAt - a.createdAt), [items, filtered, starred, search, agent, map, side]);
  const label = (list: string[][], key: string) => list.find(([id]) => id === key)?.[1] ?? key;
  async function play(item: SavedLineup) {
    setError("");
    const url = playbackUrl(item);
    if (!url) { setError(t("library.openError")); return; }
    try { await openUrl(url); } catch { setError(t("library.openError")); }
  }
  return <section className="lineup-library">
    <div className="personal-heading"><div><span className="eyebrow">{t("library.count", { n: items.length })}</span><h3>{t("library.title")}</h3><p>{t("library.hint")}</p></div>
      <button className="personal-primary" onClick={() => setEditing("new")} disabled={items.length >= LIBRARY_LIMIT}><Plus size={15}/>{t("library.add")}</button></div>
    <LineupTransfer/>
    <label className="personal-check"><input type="checkbox" checked={follow} disabled={!context&&!follow} onChange={e=>setFollow(e.target.checked)}/>{t("suite.relevant")}</label><p className="personal-muted">{context?t("suite.contextHint")+" · "+context.map+" / "+context.agent+" / "+t("lineups."+context.side):t("suite.contextMissing")}</p>
    <div className="library-toolbar"><label className="personal-search"><Search size={15}/><input ref={input} aria-label={t("library.search")} placeholder={t("library.search")} value={search} onChange={e => setSearch(e.target.value)} maxLength={120}/></label>
      <label className="personal-check"><input type="checkbox" checked={filtered} onChange={e => setFiltered(e.target.checked)}/>{t("library.matchFilters")}</label>
      <label className="personal-check"><input type="checkbox" checked={starred} onChange={e => setStarred(e.target.checked)}/>{t("library.favoritesOnly")}</label></div>
    {error && <p role="alert" className="personal-error">{error}</p>}
    {!rows.length && <div className="personal-empty"><BookMarked size={29}/><h3>{t("library.empty")}</h3><p>{t("library.emptyHint")}</p></div>}
    <div className="library-grid">{rows.map(item => <article className="library-card" key={item.id}>
      <div className="library-card-top"><span>{label(maps, item.map)} · {label(agents, item.agent)}</span><button className={"personal-icon" + (item.favorite ? " is-favorite" : "")} aria-label={t(item.favorite ? "library.unfavorite" : "library.favorite")} aria-pressed={item.favorite} onClick={() => favorite(item.id)}><Star size={16} fill={item.favorite ? "currentColor" : "none"}/></button></div>
      <h4>{item.title}</h4><small>{t("lineups." + item.side)} · {timeText(item.seconds)}</small>
      <p className="library-notes">{item.notes || t("library.noNotes")}</p>
      <div className="library-card-actions"><button onClick={() => void play(item)}><ExternalLink size={14}/>{t("library.play")}</button>
        <button className="personal-icon" aria-label={t("library.edit")} onClick={() => setEditing(item)}><Pencil size={14}/></button>
        <button className="personal-icon" aria-label={t("library.remove")} onClick={() => setConfirm(item.id)}><Trash2 size={14}/></button></div>
      {confirm === item.id && <div className="library-remove" role="alert"><span>{t("library.confirmRemove")}</span><button onClick={() => { remove(item.id); setConfirm(null); }}>{t("library.remove")}</button><button onClick={() => setConfirm(null)}>{t("tools.cancel")}</button></div>}
    </article>)}</div>
    {editing && <LineupForm item={editing === "new" ? null : editing} filters={{ agent, map, side }} agents={agents} maps={maps} onClose={() => setEditing(null)} onSave={item => { if (!save(item)) return false; setEditing(null); return true; }}/>}
  </section>;
}

function LineupForm({ item, filters, agents, maps, onClose, onSave }: {
  item: SavedLineup | null; filters: Pick<SavedLineup, "agent" | "map" | "side">;
  agents: string[][]; maps: string[][]; onClose: () => void; onSave: (item: SavedLineup) => boolean;
}) {
  const { t } = useI18n();
  const [form, setForm] = useState({ title: item?.title ?? "", url: item?.url ?? "", notes: item?.notes ?? "", agent: item?.agent ?? filters.agent,
    map: item?.map ?? filters.map, side: item?.side ?? filters.side, time: timeText(item?.seconds ?? 0) });
  const [error, setError] = useState("");
  const options = (list: string[][], current: string) => list.some(([id]) => id === current) ? list : [[current, current], ...list];
  function submit(e: React.FormEvent) {
    e.preventDefault();
    const url = videoUrl(form.url), seconds = parseTime(form.time);
    if (!url || seconds === null || !form.title.trim()) { setError(t("library.invalid")); return; }
    if (!onSave({ ...filters, ...item, title: form.title.trim(), url, notes: form.notes.trim(), agent: form.agent, map: form.map,
      side: form.side, seconds, id: item?.id ?? crypto.randomUUID(), favorite: item?.favorite ?? false, createdAt: item?.createdAt ?? Date.now() })) setError(t("library.full"));
  }
  return <Dialog title={t(item ? "library.edit" : "library.add")} onClose={onClose}>
    <form className="personal-form" onSubmit={submit}>
      <label>{t("library.videoTitle")}<input required maxLength={100} value={form.title} onChange={e => setForm({ ...form, title: e.target.value })}/></label>
      <label>{t("library.url")}<input aria-label={t("library.url")} required type="url" maxLength={2048} value={form.url} placeholder="https://www.youtube.com/watch?v=..." onChange={e => setForm({ ...form, url: e.target.value })}/><small>{t("library.providers")}</small></label>
      <div className="personal-form-grid"><label>{t("lineups.agent")}<select value={form.agent} onChange={e => setForm({ ...form, agent: e.target.value })}>{options(agents, form.agent).map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></label>
        <label>{t("lineups.map")}<select value={form.map} onChange={e => setForm({ ...form, map: e.target.value })}>{options(maps, form.map).map(([id, name]) => <option key={id} value={id}>{name}</option>)}</select></label>
        <label>{t("lineups.side")}<select value={form.side} onChange={e => setForm({ ...form, side: e.target.value === "defense" ? "defense" : "attack" })}><option value="attack">{t("lineups.attack")}</option><option value="defense">{t("lineups.defense")}</option></select></label>
        <label>{t("library.timestamp")}<input maxLength={7} value={form.time} placeholder="0:00" onChange={e => setForm({ ...form, time: e.target.value })}/></label></div>
      <small>{t("library.timestampHint")}</small>
      <label>{t("library.notes")}<textarea rows={4} maxLength={3000} value={form.notes} onChange={e => setForm({ ...form, notes: e.target.value })}/></label>
      {error && <p role="alert" className="personal-error">{error}</p>}
      <div className="personal-actions"><button type="button" onClick={onClose}>{t("tools.cancel")}</button><button className="personal-primary" type="submit">{t("tools.save")}</button></div>
    </form>
  </Dialog>;
}
