export interface OutputProbe {
  enabled: boolean; mode: "fullscreen" | "borderless" | "windowed" | "unknown";
  output: string; game_bar_installed: boolean; widget_installed: boolean; pinned: boolean; click_through: boolean;
}
export interface SpikeProbe { enabled: boolean; icon_enabled: boolean; icon_status: string; icon_similarity: number; error: string | null }
export interface DisplayProbe { width: number; height: number; scale: number; game_found: boolean }
export interface DiagnosticCheck { id: string; level: "ok" | "info" | "warn"; value: string; hint?: string; detail?: string }
export function diagnosticChecks(connection: string, output: OutputProbe | null, spike: SpikeProbe | null, display: DisplayProbe | null): DiagnosticCheck[] {
  const rows: DiagnosticCheck[] = [
    { id: "connection", level: connection === "CONNECTED" ? "ok" : "warn", value: connection === "CONNECTED" ? "diag.connection.ok" : "diag.fix", hint: connection === "CONNECTED" ? undefined : "diag.connection.help" },
    { id: "display", level: display?.game_found ? "ok" : "info", value: display?.game_found ? "diag.yes" : "diag.unknown",
      detail: display?.game_found ? display.width + " × " + display.height + " · " + Math.round(display.scale * 100) + "%" : undefined, hint: display?.game_found ? undefined : "diag.display.help" },
    { id: "mode", level: output && output.mode !== "unknown" ? "ok" : "info", value: output ? "diag." + output.mode : "diag.unknown", hint: !output || output.mode === "unknown" ? "diag.mode.help" : undefined },
    { id: "timer", level: spike?.enabled ? "ok" : "info", value: spike ? spike.enabled ? "diag.yes" : "diag.off" : "diag.unknown", hint: spike?.enabled ? undefined : "diag.timer.help" },
  ];
  const reading = spike?.icon_enabled && ["Simge izleniyor", "Simge görünüyor"].includes(spike.icon_status);
  const captureError = spike?.enabled && spike.icon_enabled && !["Simge izleniyor", "Simge görünüyor", "Bekleniyor", "Kapalı", "Oyun ön planda bekleniyor"].includes(spike.icon_status);
  rows.push({ id: "detector", level: reading ? "ok" : captureError || spike?.error ? "warn" : "info",
    value: reading ? "diag.detector.active" : captureError ? "diag.detector.failed" : "diag.detector.waiting", hint: reading ? undefined : "diag.detector.help" });
  if (output?.mode === "borderless" || output?.mode === "windowed") {
    rows.push({ id: "components", level: "info", value: "diag.optional" });
  } else if (output?.mode === "fullscreen") {
    const ready = output.game_bar_installed && output.widget_installed;
    rows.push({ id: "components", level: ready ? "ok" : "warn", value: ready ? "diag.components.ok" : "diag.fix", hint: ready ? undefined : "diag.components.help" });
    if (ready) rows.push(
      { id: "pin", level: output.pinned ? "ok" : "warn", value: output.pinned ? "diag.yes" : "diag.fix", hint: output.pinned ? undefined : "diag.pin.help" },
      { id: "clicks", level: output.click_through ? "ok" : "warn", value: output.click_through ? "diag.yes" : "diag.off", hint: output.click_through ? undefined : "diag.clicks.help" },
    );
  }
  return rows;
}
