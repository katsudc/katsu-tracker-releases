import { useMemo, useState } from "react";
import { Copy, Pencil, Plus, Search, Star, Trash2 } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { Dialog } from "../features/Dialog";
import { useProfileTools } from "./profileStore";
import { edpi, validCrosshairCode, type CrosshairProfile, type SensitivityProfile } from "./profileData";
type Kind = "crosshairs" | "sensitivity";
export function ProfileTools({ kind }: { kind: Kind }) {
  const { t, locale } = useI18n(), store = useProfileTools();
  const [search, setSearch] = useState(""), [favorites, setFavorites] = useState(false), [editing, setEditing] = useState<CrosshairProfile | SensitivityProfile | "new" | null>(null);
  const [deleting, setDeleting] = useState<string | null>(null), [message, setMessage] = useState("");
  const rows = useMemo(() => store[kind].filter(v => (!favorites || v.favorite) && (v.name + ("notes" in v ? " " + v.notes : "")).toLocaleLowerCase().includes(search.toLocaleLowerCase()))
    .sort((a, b) => Number(b.favorite) - Number(a.favorite) || b.modified - a.modified), [store, kind, favorites, search]);
  async function copy(code: string) { try { await navigator.clipboard.writeText(code); setMessage(t("suite.copied")); } catch { setMessage(t("tools.error")); } }
  return <section className="suite-tool">
    <div className="personal-heading"><div><h3>{t("suite." + kind)}</h3><p>{t(kind === "crosshairs" ? "suite.crosshairHint" : "suite.sensitivityHint")}</p></div><button className="personal-primary" onClick={() => setEditing("new")}><Plus size={14}/>{t(kind === "crosshairs" ? "suite.addCrosshair" : "suite.addSensitivity")}</button></div>
    <div className="library-toolbar"><label className="personal-search"><Search size={14}/><input aria-label={t("library.search")} value={search} onChange={e => setSearch(e.target.value)} placeholder={t("library.search")}/></label><label className="personal-check"><input type="checkbox" checked={favorites} onChange={e => setFavorites(e.target.checked)}/>{t("library.favoritesOnly")}</label></div>
    {message && <p role="status" className="personal-muted">{message}</p>}
    {!rows.length && <div className="personal-empty">{t(kind === "crosshairs" ? "suite.noCrosshairs" : "suite.noSensitivity")}</div>}
    <div className="suite-profile-list">{rows.map(p => <article key={p.id}>
      <div className="personal-heading"><h4>{p.name}</h4><div className="suite-row-actions">
        <button className={"personal-icon " + (p.favorite ? "is-favorite" : "")} aria-label={t(p.favorite ? "library.unfavorite" : "library.favorite")} aria-pressed={p.favorite} onClick={() => store.favorite(kind, p.id)}><Star size={15} fill={p.favorite ? "currentColor" : "none"}/></button>
        <button className="personal-icon" aria-label={t("library.edit")} onClick={() => setEditing(p)}><Pencil size={14}/></button><button className="personal-icon" aria-label={t("library.remove")} onClick={() => setDeleting(p.id)}><Trash2 size={14}/></button></div></div>
      {"code" in p ? <><code className="suite-code">{p.code}</code><p className="library-notes">{p.notes}</p><button className="personal-quiet" onClick={() => void copy(p.code)}><Copy size={13}/>{t("suite.copyCode")}</button></> :
        <><div className="suite-sensitivity-values"><span>{p.dpi} DPI</span><span>{p.sensitivity}</span><strong>{edpi(p.dpi, p.sensitivity)?.toFixed(2)} eDPI</strong></div>
          {!!p.history.length && <details><summary>{t("suite.changeHistory")}</summary>{p.history.slice(-8).reverse().map((h, i) => <p className="personal-muted" key={i}>{new Date(h.at).toLocaleDateString(locale === "tr" ? "tr-TR" : "en-US")} · {h.dpi} DPI · {h.sensitivity}</p>)}</details>}</>}
      <small className="personal-muted">{new Date(p.modified).toLocaleDateString(locale === "tr" ? "tr-TR" : "en-US")}</small>
    </article>)}</div>
    {deleting && <Dialog title={t("suite.confirmDelete")} onClose={() => setDeleting(null)}><div className="personal-actions"><button onClick={() => setDeleting(null)}>{t("tools.cancel")}</button><button className="personal-primary" onClick={() => { store.remove(kind, deleting); setDeleting(null); }}>{t("library.remove")}</button></div></Dialog>}
    {editing && <ProfileForm kind={kind} existing={editing === "new" ? null : editing} onClose={() => setEditing(null)}/>}
  </section>;
}
function ProfileForm({ kind, existing, onClose }: { kind: Kind; existing: CrosshairProfile | SensitivityProfile | null; onClose: () => void }) {
  const { t } = useI18n(), store = useProfileTools();
  const [name, setName] = useState(existing?.name ?? ""), [code, setCode] = useState(existing && "code" in existing ? existing.code : ""), [notes, setNotes] = useState(existing && "notes" in existing ? existing.notes : "");
  const [dpi, setDpi] = useState(existing && "dpi" in existing ? String(existing.dpi) : "800"), [sens, setSens] = useState(existing && "sensitivity" in existing ? String(existing.sensitivity) : "0.31"), [error, setError] = useState("");
  const preview = edpi(Number(dpi), Number(sens));
  function save(e: React.FormEvent) {
    e.preventDefault(); setError("");
    if (!name.trim() || (kind === "crosshairs" ? !validCrosshairCode(code.trim()) : preview === null)) { setError(t("suite.invalidInput")); return; }
    const base = { id: existing?.id ?? crypto.randomUUID(), name: name.trim(), favorite: existing?.favorite ?? false, created: existing?.created ?? Date.now(), modified: Date.now() };
    try {
      const old = existing && "dpi" in existing ? existing : null;
      const history = old?.history ?? [];
      const nextHistory = old && (old.dpi !== Number(dpi) || old.sensitivity !== Number(sens)) ? [...history, { at: old.modified, dpi: old.dpi, sensitivity: old.sensitivity }].slice(-30) : history;
      const ok = kind === "crosshairs" ? store.saveCrosshair({ ...base, code: code.trim(), notes }) : store.saveSensitivity({ ...base, dpi: Number(dpi), sensitivity: Number(sens), history: nextHistory });
      if (ok) onClose(); else setError(t("suite.limitReached"));
    } catch { setError(t("tools.error")); }
  }
  return <Dialog title={t(existing ? "library.edit" : kind === "crosshairs" ? "suite.addCrosshair" : "suite.addSensitivity")} onClose={onClose}><form className="personal-form" onSubmit={save}>
    <label>{t("suite.profileName")}<input required maxLength={80} value={name} onChange={e => setName(e.target.value)}/></label>
    {kind === "crosshairs" ? <><label>{t("suite.crosshairCode")}<textarea required rows={3} maxLength={1200} value={code} onChange={e => setCode(e.target.value)} placeholder="0;P;c;1;h;0;0b;1;1b;0"/></label><label>{t("suite.noteBody")}<textarea rows={3} maxLength={2000} value={notes} onChange={e => setNotes(e.target.value)}/></label></> :
      <><div className="personal-form-grid"><label>DPI<input required type="number" min={1} max={100000} step={1} value={dpi} onChange={e => setDpi(e.target.value)}/></label><label>{t("suite.sensValue")}<input required type="number" min={0.0001} max={100} step="any" value={sens} onChange={e => setSens(e.target.value)}/></label></div><strong className="suite-edpi">{preview === null ? "—" : Number(preview.toFixed(4))} eDPI</strong></>}
    {error && <p role="alert" className="personal-error">{error}</p>}<div className="personal-actions"><button type="button" onClick={onClose}>{t("tools.cancel")}</button><button className="personal-primary" type="submit">{t("tools.save")}</button></div>
  </form></Dialog>;
}
