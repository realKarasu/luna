// Reference pictures for the before/after preview. Any ref-*.jpg/png/webp
// dropped into src/assets is picked up at build time.
const files = import.meta.glob("../assets/ref-*.{jpg,jpeg,png,webp,JPG,PNG}", {
  eager: true,
  query: "?url",
  import: "default",
}) as Record<string, string>;

export interface RefPicture {
  id: string;
  name: string;
  url: string;
}

const pretty = (id: string) =>
  id
    .replace(/[-_]+/g, " ")
    .replace(/\b\w/g, (c) => c.toUpperCase());

export const REF_PICTURES: RefPicture[] = Object.entries(files)
  .map(([path, url]) => {
    const id = path.split("/").pop()!.replace(/^ref-/, "").replace(/\.[^.]+$/, "");
    return { id, name: pretty(id), url };
  })
  .sort((a, b) => {
    const order = ["dark", "indoor", "bright"];
    const ia = order.indexOf(a.id);
    const ib = order.indexOf(b.id);
    return (ia < 0 ? 99 : ia) - (ib < 0 ? 99 : ib) || a.id.localeCompare(b.id);
  });

export function refById(id: string): RefPicture | undefined {
  return REF_PICTURES.find((r) => r.id === id) ?? REF_PICTURES[0];
}

const imageCache = new Map<string, Promise<HTMLImageElement>>();

export function loadImage(url: string): Promise<HTMLImageElement> {
  let p = imageCache.get(url);
  if (!p) {
    p = new Promise((resolve, reject) => {
      const img = new Image();
      img.decoding = "async";
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error("Picture failed to load"));
      img.src = url;
    });
    imageCache.set(url, p);
  }
  return p;
}

const pixelCache = new Map<string, Promise<ImageData>>();

/** The picture scaled to cover width x height, as raw pixels. */
export function loadPixels(url: string, width: number, height: number): Promise<ImageData> {
  const key = `${url}@${width}x${height}`;
  let p = pixelCache.get(key);
  if (!p) {
    p = loadImage(url).then((img) => {
      const c = document.createElement("canvas");
      c.width = width;
      c.height = height;
      const ctx = c.getContext("2d", { willReadFrequently: true })!;
      const scale = Math.max(width / img.naturalWidth, height / img.naturalHeight);
      const w = img.naturalWidth * scale;
      const h = img.naturalHeight * scale;
      ctx.drawImage(img, (width - w) / 2, (height - h) / 2, w, h);
      return ctx.getImageData(0, 0, width, height);
    });
    pixelCache.set(key, p);
    if (pixelCache.size > 40) pixelCache.delete(pixelCache.keys().next().value!);
  }
  return p;
}
