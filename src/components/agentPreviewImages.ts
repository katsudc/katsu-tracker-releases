interface Entry { image: HTMLImageElement; ready: Promise<boolean> }
// Keep a small decoded working set; never preload every full-size portrait.
const images = new Map<string, Entry>();
const MAX_IMAGES = 6;

export function loadAgentPreviewImage(url: string): Promise<boolean> {
  const cached = images.get(url);
  if (cached) {
    images.delete(url);
    images.set(url, cached);
    return cached.ready;
  }
  const image = new Image();
  image.decoding = "async";
  const ready = new Promise<boolean>(resolve => {
    image.onload = () => {
      void image.decode().catch(() => {}).then(() => resolve(image.naturalWidth > 0));
    };
    image.onerror = () => { if (images.get(url)?.image === image) images.delete(url); resolve(false); };
  });
  images.set(url, { image, ready });
  if (images.size > MAX_IMAGES) images.delete(images.keys().next().value!);
  image.src = url;
  return ready;
}
