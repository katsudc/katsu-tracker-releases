export interface CrosshairProfile { id: string; name: string; code: string; notes: string; favorite: boolean; created: number; modified: number }
export interface SensitivityProfile { id: string; name: string; dpi: number; sensitivity: number; favorite: boolean; created: number; modified: number; history: Array<{ at: number; dpi: number; sensitivity: number }> }
export function validCrosshairCode(value: string): boolean {
  const tokens = value.trim().split(";");
  return value.length <= 1200 && tokens.length >= 4 && tokens.length <= 300 && tokens[0] === "0" &&
    tokens.some(v => ["P", "A", "S"].includes(v)) && tokens.every(v => /^[a-zA-Z0-9.-]{1,24}$/.test(v));
}
export function edpi(dpi: number, sensitivity: number): number | null {
  return Number.isInteger(dpi) && dpi >= 1 && dpi <= 100000 && Number.isFinite(sensitivity) && sensitivity >= 0.0001 && sensitivity <= 100 ? dpi * sensitivity : null;
}
export function validCrosshair(v: CrosshairProfile): boolean {
  return !!v && typeof v.id === "string" && /^[\w-]{1,80}$/.test(v.id) && typeof v.name === "string" && !!v.name.trim() && v.name.length <= 80 &&
    typeof v.code === "string" && validCrosshairCode(v.code) && typeof v.notes === "string" && v.notes.length <= 2000 && typeof v.favorite === "boolean" &&
    Number.isFinite(v.created) && Number.isFinite(v.modified);
}
export function validSensitivity(v: SensitivityProfile): boolean {
  return !!v && typeof v.id === "string" && /^[\w-]{1,80}$/.test(v.id) && typeof v.name === "string" && !!v.name.trim() && v.name.length <= 80 &&
    edpi(v.dpi, v.sensitivity) !== null && typeof v.favorite === "boolean" && Number.isFinite(v.created) && Number.isFinite(v.modified) &&
    Array.isArray(v.history) && v.history.length <= 30 && v.history.every(h => h && Number.isFinite(h.at) && edpi(h.dpi, h.sensitivity) !== null);
}
