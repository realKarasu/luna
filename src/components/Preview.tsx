import { useEffect, useRef, useState } from "react";
import type { ColorParams, Tone } from "../types";
import { buildLut, previewMatrix, processPixels, type HoloMode } from "../lib/math";
import { loadPixels, REF_PICTURES, refById } from "../lib/refs";

const PREVIEW_W = 960;
const PREVIEW_H = 540;

interface Look {
  tone: Tone;
  color: ColorParams;
  holo?: HoloMode;
  holoStrength?: number;
}

function render(target: HTMLCanvasElement, src: ImageData, look: Look | null) {
  const ctx = target.getContext("2d")!;
  if (!look) {
    ctx.putImageData(src, 0, 0);
    return;
  }
  const out = new ImageData(src.width, src.height);
  processPixels(src.data, out.data, previewMatrix(look.color, look.holo ?? "off", look.holoStrength ?? 0), buildLut(look.tone));
  ctx.putImageData(out, 0, 0);
}

/** Before/after picture with a draggable split. Uses the same math as the real output. */
export function Preview({
  look,
  picture,
  onPicture,
}: {
  look: Look | null;
  picture: string;
  onPicture: (id: string) => void;
}) {
  const before = useRef<HTMLCanvasElement>(null);
  const after = useRef<HTMLCanvasElement>(null);
  const box = useRef<HTMLDivElement>(null);
  const [split, setSplit] = useState(50);
  const [pixels, setPixels] = useState<ImageData | null>(null);
  const [failed, setFailed] = useState(false);
  const ref = refById(picture);

  useEffect(() => {
    if (!ref) return;
    let alive = true;
    setFailed(false);
    loadPixels(ref.url, PREVIEW_W, PREVIEW_H)
      .then((p) => alive && setPixels(p))
      .catch(() => alive && setFailed(true));
    return () => {
      alive = false;
    };
  }, [ref]);

  useEffect(() => {
    if (pixels && before.current) render(before.current, pixels, null);
  }, [pixels]);

  useEffect(() => {
    if (!pixels || !after.current) return;
    const id = requestAnimationFrame(() => after.current && render(after.current, pixels, look));
    return () => cancelAnimationFrame(id);
  }, [pixels, look]);

  const drag = (clientX: number) => {
    const r = box.current?.getBoundingClientRect();
    if (!r) return;
    setSplit(Math.min(100, Math.max(0, ((clientX - r.left) / r.width) * 100)));
  };

  return (
    <div className="preview">
      <div className="preview-pick">
        {REF_PICTURES.map((r) => (
          <button key={r.id} type="button" className={`pill ${r.id === ref?.id ? "active" : ""}`} onClick={() => onPicture(r.id)}>
            {r.name}
          </button>
        ))}
      </div>
      <div
        ref={box}
        className="preview-stage"
        onPointerDown={(e) => {
          (e.target as HTMLElement).setPointerCapture(e.pointerId);
          drag(e.clientX);
        }}
        onPointerMove={(e) => e.buttons & 1 && drag(e.clientX)}
        onKeyDown={(e) => {
          if (e.key === "ArrowLeft") setSplit((s) => Math.max(0, s - 5));
          if (e.key === "ArrowRight") setSplit((s) => Math.min(100, s + 5));
        }}
        role="slider"
        tabIndex={0}
        aria-label="Before and after split"
        aria-valuenow={Math.round(split)}
        aria-valuemin={0}
        aria-valuemax={100}
      >
        {!ref || failed ? (
          <div className="preview-empty muted">Add a picture named ref-something.jpg to the assets folder to preview here.</div>
        ) : (
          <>
            <canvas ref={after} width={PREVIEW_W} height={PREVIEW_H} className="preview-canvas" />
            <canvas
              ref={before}
              width={PREVIEW_W}
              height={PREVIEW_H}
              className="preview-canvas"
              style={{ clipPath: `inset(0 ${100 - split}% 0 0)` }}
            />
            <span className="badge left">Before</span>
            <span className="badge right">After</span>
            <div className="split-line" style={{ left: `${split}%` }}>
              <span className="split-handle">
                <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
                  <path d="M9 6l-5 6 5 6 M15 6l5 6-5 6" />
                </svg>
              </span>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

/** Small live thumbnail of a look on a reference picture. */
export function Thumb({ look, picture, width = 200, height = 112 }: { look: Look; picture: string; width?: number; height?: number }) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const ref = refById(picture);
  const key = JSON.stringify(look);
  useEffect(() => {
    if (!ref) return;
    let alive = true;
    loadPixels(ref.url, width, height).then((p) => {
      if (alive && canvas.current) render(canvas.current, p, look);
    });
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ref, key, width, height]);
  return <canvas ref={canvas} width={width} height={height} className="thumb" />;
}
