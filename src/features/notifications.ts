import { toast as nativeToast } from "sonner";
import { usePersonalStore } from "./personalStore";
const quiet = () => {
  const { stream } = usePersonalStore.getState();
  return stream.enabled && stream.quietNotifications;
};
// Drop quiet notifications at creation, so old notices do not reappear after streaming.
export const toast = {
  success: (...args: Parameters<typeof nativeToast.success>) => quiet() ? undefined : nativeToast.success(...args),
  error: (...args: Parameters<typeof nativeToast.error>) => quiet() ? undefined : nativeToast.error(...args),
  loading: (...args: Parameters<typeof nativeToast.loading>) => quiet() ? undefined : nativeToast.loading(...args),
  dismiss: nativeToast.dismiss,
};
