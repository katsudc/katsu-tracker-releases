import { useEffect, useMemo, useRef, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { BarChart3, BookOpen, Crosshair, EyeOff, Home, Keyboard, LockKeyhole, NotebookPen, Settings2, ShieldCheck, SlidersHorizontal, Timer, Wrench } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useSettingsStore } from "../stores/settingsStore";
import { usePanelStore } from "../stores/panelStore";
import { useTrackerStore } from "../tracker/store";
import { usePersonalStore } from "../features/personalStore";
import { useSuiteNavigation, type ToolTab } from "./navigation";
import { accountId } from "./storage";
import { Dialog } from "../features/Dialog";

function score(label: string, query: string): number {
  const text = label.toLocaleLowerCase(), q = query.toLocaleLowerCase().trim();
  if (!q) return 1;
  if (text.startsWith(q)) return 100;
  if (text.includes(q)) return 80 - text.indexOf(q);
  let i = 0; for (const c of text) if (c === q[i]) i++;
  return i === q.length ? 20 : 0;
}
export function CommandPalette() {
  const { t, locale } = useI18n();
  const [open, setOpen] = useState(false), [query, setQuery] = useState(""), [selected, setSelected] = useState(0);
  const input = useRef<HTMLInputElement>(null);
  useEffect(() => {
    function show() { setQuery(""); setSelected(0); setOpen(v => !v); }
    function key(e: KeyboardEvent) {
      if (!e.ctrlKey || e.altKey || e.shiftKey || e.key.toLowerCase() !== "k" || e.repeat || e.defaultPrevented) return;
      const settings = useSettingsStore.getState();
      if (settings.isHotkeyPaused || settings.hotkey.toLowerCase().replace(/control/g, "ctrl") === "ctrl+k") return;
      if (document.querySelector('[role="dialog"]') && !document.querySelector(".command-palette")) return;
      e.preventDefault(); show();
    }
    window.addEventListener("keydown", key);
    window.addEventListener("katsu-command-palette", show);
    return () => { window.removeEventListener("keydown", key); window.removeEventListener("katsu-command-palette", show); };
  }, []);
  useEffect(() => { if (open) { queueMicrotask(() => input.current?.focus()); } }, [open]);
  const commands = useMemo(() => {
    const go = (view: string, tool?: ToolTab, focus?: string) => () => {
      useTrackerStore.getState().select(null); useSuiteNavigation.getState().go(view, tool, focus); void usePanelStore.getState().close();
    };
    return [
      { id: "match", text: t("desktop.matchCenter"), Icon: Home, run: go("match") },
      { id: "stats", text: t("tracker.statistics"), Icon: BarChart3, run: go("statistics") },
      { id: "instalock", text: t("instalock.title"), Icon: LockKeyhole, run: go("instalock") },
      { id: "spike", text: t("spike.title"), Icon: Timer, run: go("spike") },
      { id: "tools", text: t("tools.open"), Icon: Wrench, run: go("tools", "home") },
      { id: "lineups", text: t("library.search"), Icon: BookOpen, run: go("tools", "lineups", "lineup-search") },
      { id: "crosshair", text: t("suite.crosshairs"), Icon: Crosshair, run: go("tools", "crosshairs") },
      { id: "sens", text: t("suite.sensitivity"), Icon: SlidersHorizontal, run: go("tools", "sensitivity") },
      { id: "notes", text: t("suite.notes"), Icon: NotebookPen, run: go("tools", "notes") },
      { id: "diagnostics", text: t("diag.title"), Icon: Wrench, run: go("tools", "diagnostics") },
      { id: "privacy", text: t("suite.privacy"), Icon: ShieldCheck, run: go("tools", "privacy") },
      { id: "settings", text: t("settings.title"), Icon: Settings2, run: () => { void usePanelStore.getState().openSettings(); } },
      { id: "stream", text: t("stream.title"), Icon: EyeOff, run: () => { const s = usePersonalStore.getState(); s.setStream({ enabled: !s.stream.enabled }); } },
      { id: "session", text: t("session.reset"), Icon: Timer, run: async () => {
        const id = useTrackerStore.getState().data.profile?.puuid;
        if (!accountId(id)) return;
        if (await confirm(t("session.confirm"), { title: t("session.reset"), kind: "info" })) {
          usePersonalStore.getState().startSession(id); go("statistics")();
        }
      } },
    ];
  }, [t, locale]);
  const results = useMemo(() => !open ? [] : commands.map(v => ({ ...v, score: score(v.text + " " + v.id, query) })).filter(v => v.score > 0).sort((a, b) => b.score - a.score), [commands, query, open]);
  useEffect(() => { setSelected(0); }, [query]);
  useEffect(() => { document.querySelector('[data-command-selected="true"]')?.scrollIntoView({ block: "nearest" }); }, [selected]);
  if (!open) return null;
  function run(index: number) { const command = results[index]; if (!command) return; setOpen(false); void command.run(); }
  return <Dialog title={t("suite.palette")} onClose={() => setOpen(false)}>
    <div className="command-palette" onKeyDown={e => {
      if (["ArrowDown", "ArrowUp", "Enter"].includes(e.key)) e.preventDefault();
      if (e.key === "ArrowDown") setSelected(v => Math.min(results.length - 1, v + 1));
      if (e.key === "ArrowUp") setSelected(v => Math.max(0, v - 1));
      if (e.key === "Enter") run(selected);
    }}>
      <label className="personal-search"><Keyboard size={16}/><input ref={input} role="combobox" aria-label={t("suite.paletteSearch")} aria-expanded={true} aria-controls="suite-command-results" aria-activedescendant={results[selected] ? "command-" + results[selected].id : undefined} value={query} onChange={e => setQuery(e.target.value)} placeholder={t("suite.paletteSearch")} autoComplete="off"/></label>
      <div role="listbox" id="suite-command-results" className="command-results">{results.map((command, i) => <button id={"command-" + command.id} key={command.id} role="option" aria-selected={i === selected} data-command-selected={i === selected} onMouseEnter={() => setSelected(i)} onClick={() => run(i)}><command.Icon size={17}/><span>{command.text}</span></button>)}
        {!results.length && <p className="personal-empty">{t("suite.noCommands")}</p>}</div>
      <small className="personal-muted">{t("suite.paletteHint")}</small>
    </div>
  </Dialog>;
}
