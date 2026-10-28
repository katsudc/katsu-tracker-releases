import { useEffect, useRef, useState } from "react";
import { ChevronDown, Music2, Pause, Play, SkipBack, SkipForward, Volume2 } from "lucide-react";
import { invokeCommand } from "../utils/ipc";
import { useI18n } from "../lib/i18n";

interface Media {
  connected: boolean; title: string; artist: string; album: string; cover: string | null;
  playing: boolean; can_toggle: boolean; can_next: boolean; can_previous: boolean;
  volume: number | null; position_seconds?: number | null; duration_seconds?: number | null;
}
const EMPTY: Media = { connected: false, title: "", artist: "", album: "", cover: null, playing: false, can_toggle: false, can_next: false, can_previous: false, volume: null };
function timeLabel(seconds: number) {
  const value = Math.max(0, Math.floor(seconds));
  return `${Math.floor(value / 60)}:${String(value % 60).padStart(2, "0")}`;
}

export function MusicIsland() {
  const { t } = useI18n();
  const [media, setMedia] = useState(EMPTY);
  const [expanded, setExpanded] = useState(false);
  const [busy, setBusy] = useState(false);
  const [volume, setVolume] = useState(0);
  const [error, setError] = useState("");
  const [position, setPosition] = useState(0);
  const volumeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const closeTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const editingUntil = useRef(0);
  const receivedAt = useRef(performance.now());
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let disposed = false, inFlight = false;
    const poll = async () => {
      if (inFlight) return;
      inFlight = true;
      try {
        const next = await invokeCommand<Media>("media_status", undefined, { suppressErrorToast: true });
        if (!disposed && next) {
          receivedAt.current = performance.now();
          setMedia(next);
          setPosition(next.position_seconds ?? 0);
          if (Date.now() > editingUntil.current) setVolume(Math.round((next.volume ?? 0) * 100));
        }
      } catch { /* The disconnected state remains usable while native media reconnects. */ }
      finally { inFlight = false; }
    };
    void poll();
    const timer = setInterval(() => void poll(), 1500);
    return () => { disposed = true; clearInterval(timer); clearTimeout(volumeTimer.current); clearTimeout(closeTimer.current); };
  }, []);

  useEffect(() => {
    if (!expanded || !media.playing || media.position_seconds == null || !media.duration_seconds) return;
    const timer = setInterval(() => setPosition(Math.min(media.duration_seconds!, media.position_seconds! + (performance.now() - receivedAt.current) / 1000)), 250);
    return () => clearInterval(timer);
  }, [expanded, media]);

  async function control(action: string, value?: number) {
    if (busy && action !== "volume") return;
    setBusy(true); setError("");
    try {
      await invokeCommand("media_control", { action, volume: value ?? null }, { suppressErrorToast: true });
      if (action === "toggle") setMedia(m => ({ ...m, playing: !m.playing }));
    } catch {
      setError(t(action === "volume" ? "music.error.volume" : "music.error.control"));
    } finally { setBusy(false); }
  }
  function changeVolume(value: number) {
    setVolume(value); editingUntil.current = Date.now() + 1500;
    clearTimeout(volumeTimer.current);
    volumeTimer.current = setTimeout(() => void control("volume", value / 100), 120);
  }
  function open() { clearTimeout(closeTimer.current); setExpanded(true); }
  function leave() {
    closeTimer.current = setTimeout(() => { if (!root.current?.querySelector(":focus-visible")) setExpanded(false); }, 140);
  }
  const duration = media.duration_seconds ?? 0;
  const hasTimeline = duration > 0 && media.position_seconds != null;

  return (
    <div ref={root} className={`music-island ${expanded ? "is-expanded" : ""}`} onMouseEnter={open} onMouseLeave={leave} onFocusCapture={open} onBlurCapture={e => { if (!e.currentTarget.contains(e.relatedTarget as Node)) setExpanded(false); }} onKeyDown={e => { if (e.key === "Escape") { setExpanded(false); (document.activeElement as HTMLElement)?.blur(); } }}>
      {media.cover && <img className="music-backdrop" src={media.cover} alt="" aria-hidden/>}
      <button className="music-summary" onClick={open} aria-expanded={expanded} aria-label={t("music.controls")}>
        <span className="music-art">{media.cover ? <img src={media.cover} alt=""/> : <Music2 size={20}/>}</span>
        <span className="music-copy"><span className="music-service">Spotify <span>· {media.connected ? media.playing ? t("music.playing") : t("music.paused") : t("music.waiting")}</span></span><strong title={media.title}>{media.connected ? media.title || t("music.untitled") : t("music.idleTitle")}</strong><span className="music-artist">{media.connected ? media.artist : t("music.idleDescription")}</span></span>
        <span className={`music-equalizer ${media.playing ? "is-playing" : ""}`} aria-hidden><i/><i/><i/></span>
        <ChevronDown className="music-chevron" size={13} aria-hidden/>
      </button>
      {expanded && <div className="music-expanded">
        {hasTimeline && <div className="music-timeline" aria-label={t("music.progress")}><div role="progressbar" aria-valuenow={Math.floor(position)} aria-valuemin={0} aria-valuemax={Math.floor(duration)} aria-valuetext={`${timeLabel(position)} / ${timeLabel(duration)}`}><i style={{ width: `${Math.min(100, position / duration * 100)}%` }}/></div><span>{timeLabel(position)}</span><span>{timeLabel(duration)}</span></div>}
        <div className="music-transport">
          <button aria-label={t("music.previous")} title={t("music.previous")} disabled={!media.can_previous || busy} onClick={() => void control("previous")}><SkipBack size={19} fill="currentColor"/></button>
          <button className="music-play" aria-label={media.playing ? t("music.pause") : t("music.play")} disabled={!media.can_toggle || busy} onClick={() => void control("toggle")}>{media.playing ? <Pause size={20} fill="currentColor"/> : <Play size={20} fill="currentColor"/>}</button>
          <button aria-label={t("music.next")} title={t("music.next")} disabled={!media.can_next || busy} onClick={() => void control("next")}><SkipForward size={19} fill="currentColor"/></button>
        </div>
        <label className="music-volume" title={t("music.volumeHint")}><Volume2 size={16}/><input aria-label={t("music.volume")} type="range" min="0" max="100" value={volume} disabled={media.volume === null} onChange={e => changeVolume(Number(e.target.value))}/><span>{media.volume === null ? "—" : `${volume}%`}</span></label>
        {error && <p className="music-error" role="status">{error}</p>}
      </div>}
    </div>
  );
}
