import { invoke as nativeInvoke, type InvokeArgs, type InvokeOptions } from "@tauri-apps/api/core";
const guarded: Record<string, string> = {
  spike_get_status: "guarded_spike_get_status", get_spike_output_status: "guarded_get_spike_output_status",
  media_status: "guarded_media_status", media_control: "guarded_media_control", set_discord_rpc: "guarded_set_discord_rpc",
};
// Original command implementations remain untouched; these adapters tolerate optional modules not being started.
export function invoke<T>(command: string, args?: InvokeArgs, options?: InvokeOptions): Promise<T> {
  return nativeInvoke<T>(guarded[command] ?? command, args, options);
}
