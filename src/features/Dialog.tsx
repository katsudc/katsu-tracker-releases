import { useEffect, useRef, type ReactNode } from "react";
import { createPortal } from "react-dom";
import { X } from "lucide-react";
import { useI18n } from "../lib/i18n";

export function Dialog({ title, onClose, children, wide = false, dismissible = true }: {
  title: string; onClose: () => void; children: ReactNode; wide?: boolean; dismissible?: boolean;
}) {
  const { t } = useI18n();
  const root = useRef<HTMLElement>(null);
  const close = useRef(onClose);
  close.current = onClose;
  useEffect(() => {
    const before = document.activeElement as HTMLElement | null;
    const target = root.current;
    const focusable = () => Array.from(target?.querySelectorAll<HTMLElement>('button:not(:disabled),input:not(:disabled),select:not(:disabled),textarea:not(:disabled),[tabindex="0"]') ?? []).filter(el => el.getClientRects().length);
    (focusable()[0] ?? target)?.focus();
    function keys(e: KeyboardEvent) {
      if (e.key === "Escape") { e.preventDefault(); e.stopPropagation(); close.current(); }
      if (e.key === "Tab") {
        const controls = focusable(), first = controls[0], last = controls[controls.length - 1];
        if (!first) { e.preventDefault(); return; }
        if (e.shiftKey && (document.activeElement === first || !target?.contains(document.activeElement))) { e.preventDefault(); last.focus(); }
        else if (!e.shiftKey && (document.activeElement === last || !target?.contains(document.activeElement))) { e.preventDefault(); first.focus(); }
      }
    }
    document.addEventListener("keydown", keys, true);
    return () => { document.removeEventListener("keydown", keys, true); if (before?.isConnected) before.focus(); };
  }, []);
  return createPortal(<div className="personal-dialog-backdrop">
    <section ref={root} tabIndex={-1} className={"personal-dialog" + (wide ? " is-wide" : "")} role="dialog" aria-modal="true" aria-label={title}>
      <header><h2>{title}</h2>{dismissible && <button className="personal-icon" type="button" aria-label={t("tools.close")} onClick={onClose}><X size={19}/></button>}</header>
      {children}
    </section>
  </div>, document.body);
}
