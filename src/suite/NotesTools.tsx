import { useEffect, useMemo, useState } from "react";
import { Pencil, Plus, Search, Trash2 } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useTrackerStore } from "../tracker/store";
import { AGENTS } from "../lib/constants";
import { LINEUP_MAPS } from "../stores/lineupsStore";
import { useMarkerConfig, validShortcut } from "./markerConfig";
import { useMarkerStore } from "./markerStore";
import { useNoteStore, type PersonalNote } from "./noteStore";
import { accountId } from "./storage";
import { Dialog } from "../features/Dialog";
import { useSuiteNavigation } from "./navigation";
export function NotesTools() {
  const { t, locale } = useI18n(), data = useTrackerStore(s => s.data);
  const account = data.profile?.puuid;
  const { notes, remove } = useNoteStore(), markers = useMarkerStore(), config = useMarkerConfig();
  const [tab, setTab] = useState<"match" | "reference" | "moments">("match"), [search, setSearch] = useState("");
  const [editing, setEditing] = useState<PersonalNote | "new" | null>(null), [deleting, setDeleting] = useState<{ kind: string; id: string } | null>(null);
  const [key, setKey] = useState(config.hotkey), [keyError, setKeyError] = useState("");
  useEffect(()=>{setEditing(null);setDeleting(null);},[account]);
  const focus = useSuiteNavigation(s => s.focus);
  useEffect(() => { if (focus === "moments") { setTab("moments"); useSuiteNavigation.getState().consumeFocus(); } }, [focus]);
  const matchLabel = (id: string | null) => { const m = data.matches.find(v => v.id === id); return m ? m.map + " · " + m.me.agent : t("suite.match"); };
  const rows = useMemo(() => notes.filter(v => (tab === "match" ? v.kind === "match" && v.account === account : v.kind === "reference") &&
    (v.body + " " + v.map + " " + v.agent).toLocaleLowerCase().includes(search.toLocaleLowerCase())), [notes, tab, account, search]);
  const moments = markers.moments.filter(v => v.account === account && v.note.toLocaleLowerCase().includes(search.toLocaleLowerCase())).slice().reverse();
  function saveKey(e: React.FormEvent) { e.preventDefault(); if (!validShortcut(key)) { setKeyError(t("suite.invalidInput")); return; } config.set(config.enabled, key); setKeyError(""); }
  return <section className="suite-tool">
    <div className="personal-heading"><div><h3>{t("suite.notes")}</h3><p>{t("suite.sharedReference")}</p></div>{tab !== "moments" && <button className="personal-primary" disabled={tab === "match" && !accountId(account)} onClick={() => setEditing("new")}><Plus size={14}/>{t("suite.addNote")}</button>}</div>
    <div className="personal-tabs">{(["match", "reference", "moments"] as const).map(v => <button aria-pressed={tab === v} key={v} onClick={() => setTab(v)}>{t(v === "match" ? "suite.matchNotes" : v === "reference" ? "suite.referenceNotes" : "suite.moments")}</button>)}</div>
    <label className="personal-search suite-search"><Search size={14}/><input aria-label={t("library.search")} value={search} onChange={e => setSearch(e.target.value)} placeholder={t("library.search")}/></label>
    {!accountId(account) && tab !== "reference" && <p className="personal-muted">{t("suite.noAccount")}</p>}
    {tab === "moments" ? <>
      <p className="personal-muted">{t("suite.markerHint")}</p>
      <form className="suite-marker-config" onSubmit={saveKey}><label className="personal-check"><input type="checkbox" checked={config.enabled} onChange={e => config.set(e.target.checked, config.hotkey)}/>{t("suite.markMoment")}</label>
        <label>{t("suite.markerKey")}<input value={key} maxLength={60} onChange={e => setKey(e.target.value)} aria-label={t("suite.markerKey")}/></label><button className="personal-quiet" type="submit">{t("tools.save")}</button></form>
      {keyError && <p className="personal-error" role="alert">{keyError}</p>}
      {!moments.length && <div className="personal-empty">{t("suite.noMoments")}</div>}
      {moments.map(v => <article className="suite-note-row" key={v.id}><div className="personal-heading"><strong>{matchLabel(v.match)} · {new Date(v.at).toLocaleTimeString(locale === "tr" ? "tr-TR" : "en-US")}</strong><button className="personal-icon" aria-label={t("library.remove")} onClick={() => setDeleting({ kind: "moment", id: v.id })}><Trash2 size={14}/></button></div>
        <p className="personal-muted">{v.round === null ? t("suite.roundUnknown") : String(v.round)}{v.observedElapsed !== null && " · " + t("suite.observedTime", { time: Math.floor(v.observedElapsed / 60000) + ":" + String(Math.floor(v.observedElapsed / 1000) % 60).padStart(2, "0") })}</p>
        <textarea className="suite-inline-note" aria-label={t("suite.noteBody")} defaultValue={v.note} maxLength={3000} rows={2} onBlur={e => markers.note(v.id, e.target.value)}/></article>)}
    </> : <>{!rows.length && <div className="personal-empty">{t("suite.noNotes")}</div>}{rows.map(v => <article className="suite-note-row" key={v.id}>
      <div className="personal-heading"><strong>{v.kind === "match" ? matchLabel(v.match) : (v.map || t("suite.anyMap")) + " · " + (v.agent || t("suite.anyAgent"))}</strong><div className="suite-row-actions"><button className="personal-icon" aria-label={t("suite.editNote")} onClick={() => setEditing(v)}><Pencil size={14}/></button><button className="personal-icon" aria-label={t("library.remove")} onClick={() => setDeleting({ kind: "note", id: v.id })}><Trash2 size={14}/></button></div></div><p className="library-notes">{v.body}</p><small className="personal-muted">{new Date(v.modified).toLocaleDateString(locale === "tr" ? "tr-TR" : "en-US")}</small>
    </article>)}</>}
    {editing && <NoteForm existing={editing === "new" ? null : editing} kind={tab === "reference" ? "reference" : "match"} onClose={() => setEditing(null)}/>}
    {deleting && <Dialog title={t("suite.confirmDelete")} onClose={() => setDeleting(null)}><div className="personal-actions"><button onClick={() => setDeleting(null)}>{t("tools.cancel")}</button><button className="personal-primary" onClick={() => { if (deleting.kind === "note") remove(deleting.id); else markers.remove(deleting.id); setDeleting(null); }}>{t("library.remove")}</button></div></Dialog>}
  </section>;
}
function NoteForm({ existing, kind, onClose }: { existing: PersonalNote | null; kind: "match" | "reference"; onClose: () => void }) {
  const { t } = useI18n(), data = useTrackerStore(s => s.data), saveNote = useNoteStore(s => s.save);
  const [body, setBody] = useState(existing?.body ?? ""), [match, setMatch] = useState(existing?.match ?? data.matches[0]?.id ?? "");
  const [agent, setAgent] = useState(existing?.agent ?? ""), [map, setMap] = useState(existing?.map ?? ""), [error, setError] = useState("");
  function save(e: React.FormEvent) {
    e.preventDefault();
    if (!body.trim() || (kind === "match" && (!accountId(data.profile?.puuid) || !match))) { setError(t("suite.invalidInput")); return; }
    const selected = data.matches.find(m => m.id === match);
    try {
      const ok = saveNote({ id: existing?.id ?? crypto.randomUUID(), kind, body: body.trim(), modified: Date.now(),
        account: kind === "match" ? data.profile!.puuid : null, match: kind === "match" ? match : null,
        agent: kind === "match" ? selected?.me.agent ?? "" : agent, map: kind === "match" ? selected?.map ?? "" : map });
      if (ok) onClose(); else setError(t("suite.limitReached"));
    } catch { setError(t("tools.error")); }
  }
  return <Dialog title={t(existing ? "suite.editNote" : "suite.addNote")} onClose={onClose}><form className="personal-form" onSubmit={save}>
    {kind === "match" ? <label>{t("suite.match")}<select value={match} onChange={e => setMatch(e.target.value)}>{existing?.match && !data.matches.some(m => m.id === existing.match) && <option value={existing.match}>{existing.map} · {existing.agent}</option>}{data.matches.map(m => <option key={m.id} value={m.id}>{m.map} · {m.me.agent} · {m.ally_score ?? "—"}:{m.enemy_score ?? "—"}</option>)}</select></label> :
      <div className="personal-form-grid"><label>{t("lineups.agent")}<select value={agent} onChange={e => setAgent(e.target.value)}><option value="">{t("suite.anyAgent")}</option>{AGENTS.map(v => <option key={v}>{v}</option>)}</select></label><label>{t("lineups.map")}<select value={map} onChange={e => setMap(e.target.value)}><option value="">{t("suite.anyMap")}</option>{LINEUP_MAPS.map(v => <option key={v}>{v}</option>)}</select></label></div>}
    <label>{t("suite.noteBody")}<textarea required rows={5} maxLength={4000} value={body} onChange={e => setBody(e.target.value)}/></label>{error && <p className="personal-error" role="alert">{error}</p>}<div className="personal-actions"><button type="button" onClick={onClose}>{t("tools.cancel")}</button><button className="personal-primary" type="submit">{t("tools.save")}</button></div>
  </form></Dialog>;
}
export function MatchNoteBlock({ account, match, map, agent }: { account: string; match: string; map: string; agent: string }) {
  const { t } = useI18n(), store = useNoteStore();
  const existing = store.notes.find(n => n.kind === "match" && n.account === account && n.match === match);
  const [body, setBody] = useState(existing?.body ?? ""), [message, setMessage] = useState("");
  const moments = useMarkerStore(s => s.moments.filter(m => m.account === account && m.match === match).length);
  useEffect(() => { setBody(existing?.body ?? ""); }, [existing?.body, account, match]);
  return <section className="suite-match-note"><div className="personal-heading"><h3>{t("suite.matchNotes")}</h3><button className="personal-quiet" onClick={() => useSuiteNavigation.getState().go("tools", "notes", "moments")}>{t("suite.moments")} · {moments}</button></div>
    <textarea className="suite-inline-note" rows={3} maxLength={4000} aria-label={t("suite.noteBody")} value={body} onChange={e => setBody(e.target.value)}/>
    <div className="personal-actions"><button disabled={!accountId(account)} onClick={() => {
      try {
        if (!body.trim()) { if (existing) store.remove(existing.id); }
        else if (!store.save({ id: existing?.id ?? crypto.randomUUID(), account, match, map, agent, kind: "match", body: body.trim(), modified: Date.now() })) throw new Error("storage");
        setMessage(t("tools.saved"));
      } catch { setMessage(t("tools.error")); }
    }}>{t("tools.save")}</button><small role="status">{message}</small></div>
  </section>;
}
