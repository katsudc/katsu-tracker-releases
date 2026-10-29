import { useEffect, useRef, useState } from "react";
import { Copy, Download, Share2 } from "lucide-react";
import { useI18n } from "../lib/i18n";
import { useSettingsStore } from "../stores/settingsStore";
import { acs, adr, hs, outcome, type TrackerMatch } from "../tracker/models";
import { usePersonalStore } from "./personalStore";
import { Dialog } from "./Dialog";
import { cardColors as colors, outputCard } from "./imageExport";

const metric = (n: number | null, digits = 0) => n == null ? "—" : n.toFixed(digits);
export function MatchShareButton({ match }: { match: TrackerMatch }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  return <><button className="personal-share-button" onClick={() => setOpen(true)}><Share2 size={14}/>{t("share.open")}</button>{open && <MatchShare match={match} onClose={() => setOpen(false)}/>}</>;
}
export function MatchShare({ match, onClose }: { match: TrackerMatch; onClose: () => void }) {
  const { t, locale } = useI18n();
  const theme = useSettingsStore(s => s.theme);
  const hidden = usePersonalStore(s => s.stream.enabled && s.stream.hideNames);
  const canvas = useRef<HTMLCanvasElement>(null);
  const [name, setName] = useState(false), [busy, setBusy] = useState(false), [ready, setReady] = useState(false);
  const [message, setMessage] = useState(""), [error, setError] = useState("");
  useEffect(() => { if (hidden) setName(false); }, [hidden]);
  useEffect(() => {
    let live = true;
    setReady(false);
    async function draw() {
      try {
        await document.fonts.ready;
        if (!live || !canvas.current) return;
        const c = canvas.current, ctx = c.getContext("2d");
        if (!ctx) throw new Error("canvas");
        c.width = 1080; c.height = 660;
        const accent = colors[theme];
        const gradient = ctx.createLinearGradient(0, 0, 1080, 660);
        gradient.addColorStop(0, "#0d1726"); gradient.addColorStop(1, "#080d16");
        ctx.fillStyle = gradient; ctx.fillRect(0, 0, 1080, 660);
        ctx.strokeStyle = "#ffffff09"; ctx.lineWidth = 1;
        for (let x = 0; x < 1080; x += 54) { ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, 660); ctx.stroke(); }
        for (let y = 0; y < 660; y += 54) { ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(1080, y); ctx.stroke(); }
        ctx.fillStyle = accent; ctx.fillRect(48, 48, 5, 38);
        function text(value: string, x: number, y: number, size: number, color = "#edf3ff", weight = 600, width = 950) {
          ctx!.font = weight + " " + size + "px 'Chakra Petch', 'Segoe UI', sans-serif";
          ctx!.fillStyle = color; ctx!.fillText(value, x, y, width);
        }
        text("katsu's Tracker", 72, 77, 28);
        text(t("share.personal"), 48, 133, 14, "#91a4c0", 500);
        const result = outcome(match);
        text(t(result === "win" ? "lastMatch.victory" : result === "loss" ? "lastMatch.defeat" : result === "draw" ? "lastMatch.draw" : "tracker.unavailable").toLocaleUpperCase(locale), 48, 211, 44, accent, 700);
        text((match.ally_score ?? "—") + " : " + (match.enemy_score ?? "—"), 48, 313, 82, "#ffffff", 700);
        text(match.map, 570, 224, 38, "#edf3ff", 600, 450);
        text(match.me.agent, 570, 268, 23, "#9bacc4", 500, 450);
        text(name && !hidden ? match.me.name ?? t("share.you") : t("share.you"), 570, 312, 22, accent, 600, 450);
        ctx.strokeStyle = "#ffffff18"; ctx.beginPath(); ctx.moveTo(48, 355); ctx.lineTo(1032, 355); ctx.stroke();
        const values = [
          ["K / D / A", (match.me.kills ?? "—") + " / " + (match.me.deaths ?? "—") + " / " + (match.me.assists ?? "—")],
          ["ACS", metric(acs(match.me))], ["HS%", metric(hs(match.me)) + (hs(match.me) == null ? "" : "%")], ["ADR", metric(adr(match.me))],
        ];
        values.forEach(([label, value], i) => { const x = 48 + i * 252; text(label, x, 404, 16, "#91a4c0", 500); text(value, x, 458, i === 0 ? 32 : 38); });
        text(match.started_at ? new Date(match.started_at).toLocaleDateString(locale === "tr" ? "tr-TR" : "en-US", { day: "numeric", month: "long", year: "numeric" }) : "—", 48, 583, 18, "#91a4c0", 500);
        text(t("queue." + match.queue.toLowerCase()).startsWith("queue.") ? match.queue : t("queue." + match.queue.toLowerCase()), 48, 615, 14, "#697d9a", 500);
        text("KATSU", 895, 602, 24, accent, 700);
        if (live) setReady(true);
      } catch { if (live) setError(t("share.error")); }
    }
    void draw();
    return () => { live = false; };
  }, [match, name, hidden, theme, locale, t]);
  async function output(copy: boolean) {
    if (!ready || !canvas.current) return;
    setBusy(true); setMessage(""); setError("");
    try { if (await outputCard(canvas.current,"katsu-match-"+(match.started_at??Date.now())+".png",copy)) setMessage(t(copy?"share.copied":"share.saved")); }
    catch { setError(t("share.error")); } finally { setBusy(false); }
  }
  return <Dialog title={t("share.title")} onClose={() => { if (!busy) onClose(); }} wide>
    <p className="personal-muted">{t("share.description")}</p>
    <canvas ref={canvas} className="match-share-canvas" role="img" aria-label={t("share.preview")}/>
    <label className="personal-check"><input type="checkbox" disabled={hidden} checked={name && !hidden} onChange={e => setName(e.target.checked)}/>{t("share.name")}</label>
    {hidden && <p className="personal-muted">{t("share.privacy")}</p>}
    <div className="personal-actions"><button disabled={busy || !ready} onClick={() => void output(true)}><Copy size={15}/>{t("share.copy")}</button><button className="personal-primary" disabled={busy || !ready} onClick={() => void output(false)}><Download size={15}/>{t("share.save")}</button></div>
    {message && <p className="personal-success" role="status">{message}</p>}{error && <p className="personal-error" role="alert">{error}</p>}
  </Dialog>;
}
