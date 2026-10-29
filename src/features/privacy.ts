import { usePersonalStore } from "./personalStore";
import { useI18n } from "../lib/i18n";
export const usePrivateNames = () => usePersonalStore(s => s.stream.enabled && s.stream.hideNames);
export function privateName(isMe: boolean, slot = 1) {
  return useI18n.getState().t(isMe ? "stream.you" : "stream.player", { n: slot });
}
