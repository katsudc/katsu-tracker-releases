import { isTauri } from "@tauri-apps/api/core";
import { open, save } from "@tauri-apps/plugin-dialog";
import { readTextFile, stat, writeFile } from "@tauri-apps/plugin-fs";
import { SETTINGS_FILE_LIMIT } from "./preferenceData";

export async function saveUserFile(name: string, bytes: Uint8Array, kind: "json" | "png" | "zip"): Promise<boolean> {
  if (isTauri()) {
    const path = await save({ defaultPath: name, filters: [{ name: kind.toUpperCase(), extensions: [kind] }] });
    if (!path) return false;
    await writeFile(path, bytes);
  } else {
    const url = URL.createObjectURL(new Blob([bytes as BlobPart], { type: kind === "png" ? "image/png" : kind === "zip" ? "application/zip" : "application/json" }));
    const link = document.createElement("a"); link.href = url; link.download = name; link.click();
    setTimeout(() => URL.revokeObjectURL(url), 60000);
  }
  return true;
}
export async function chooseSettingsFile(limit = SETTINGS_FILE_LIMIT, label = "katsu settings"): Promise<string | null> {
  if (isTauri()) {
    const path = await open({ multiple: false, directory: false, filters: [{ name: label, extensions: ["json"] }] });
    if (!path || typeof path !== "string") return null;
    if ((await stat(path)).size > limit) throw new Error("invalid");
    return readTextFile(path);
  }
  return new Promise((resolve, reject) => {
    const input = document.createElement("input"); input.type = "file"; input.accept = ".json,application/json";
    input.oncancel = () => resolve(null);
    input.onchange = () => {
      const file = input.files?.[0]; if (!file) { resolve(null); return; }
      if (file.size > limit) { reject(new Error("invalid")); return; }
      file.text().then(resolve, reject);
    };
    input.click();
  });
}
