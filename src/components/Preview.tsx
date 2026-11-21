import { useEffect, useMemo, useRef, useState } from "react";
import type { ColorParams, Tone } from "../types";
import {
  buildLut,
  previewMatrix,
  processPixels,
  type HoloMode,
} from "../lib/math";
import { loadPixels, REF_PICTURES, refById } from "../lib/refs";
import { useStore } from "../store";

const PREVIEW_W = 960;
const PREVIEW_H = 540;

interface Look {
  tone: Tone;
  color: ColorParams;
  holo?: HoloMode;
  holoStrength?: number;
}

function render(
  target: HTMLCanvasElement,
  src: ImageData,
  look: Look | null,
): ImageData {
  const ctx = target.getContext("2d")!;
  if (!look) {
    ctx.putImageData(src, 0, 0);
    return src;
  }
  const out = new ImageData(src.width, src.height);
  processPixels(
    src.data,
    out.data,
    previewMatrix(look.color, look.holo ?? "off", look.holoStrength ?? 0),
    buildLut(look.tone),
  );
  ctx.putImageData(out, 0, 0);
  return out;
}

/** Runs a few flat colours through the same pipeline as the picture. */
function processColors(colors: number[][], look: Look | null): number[][] {
  const src = new Uint8ClampedArray(colors.length * 4);
  colors.forEach(([r, g, b], i) => src.set([r, g, b, 255], i * 4));
  if (!look) return colors;
  const out = new Uint8ClampedArray(src.length);
  processPixels(
    src,
    out,
    previewMatrix(look.color, look.holo ?? "off", look.holoStrength ?? 0),
    buildLut(look.tone),
  );
  return colors.map((_, i) => [out[i * 4], out[i * 4 + 1], out[i * 4 + 2]]);
}

const css = ([r, g, b]: number[]) => `rgb(${r}, ${g}, ${b})`;
const luma = ([r, g, b]: number[]) =>
  Math.round(0.2126 * r + 0.7152 * g + 0.0722 * b);

/** Calibration skin: an 11 step grey wedge, input above and output below, plus channel ramps. */
function StepWedge({ look }: { look: Look | null }) {
  const steps = useMemo(
    () => Array.from({ length: 11 }, (_, i) => Math.round(i * 25.5)),
    [],
  );
  const out = useMemo(
    () =>
      processColors(
        steps.map((v) => [v, v, v]),
        look,
      ),
    [steps, look],
  );
  const ramps = useMemo(
    () =>
      processColors(
        [
          [0, 0, 0],
          [255, 0, 0],
          [0, 255, 0],
          [0, 0, 255],
        ],
        look,
      ),
    [look],
  );
  return (
    <div className="wedge">
      <div className="wedge-head">
        <span>Grey step wedge</span>
        <span className="mono muted">input over output</span>
      </div>
      <div className="wedge-row">
        {steps.map((v) => (
          <span key={v} style={{ background: css([v, v, v]) }} />
        ))}
      </div>
      <div className="wedge-row">
        {out.map((c, i) => (
          <span key={i} style={{ background: css(c) }} />
        ))}
      </div>
      <div className="wedge-labels mono">
        {out.map((c, i) => (
          <span key={i}>{luma(c)}</span>
        ))}
      </div>
      <div className="ramps">
        {(["R", "G", "B"] as const).map((name, i) => (
          <div key={name} className="ramp">
            <span className="mono">{name}</span>
            <i
              style={{
                background: `linear-gradient(90deg, ${css(ramps[0])}, ${css(ramps[i + 1])})`,
              }}
            />
            <span className="mono muted">{ramps[i + 1][i]}</span>
          </div>
        ))}
      </div>
    </div>
  );
}

const BINS = 64;

function histogram(img: ImageData): number[][] {
  const h = [0, 1, 2, 3].map(() => new Array(BINS).fill(0));
  const d = img.data;
  // Every 4th pixel is plenty for a 960x540 picture.
  for (let i = 0; i < d.length; i += 16) {
    const r = d[i],
      g = d[i + 1],
      b = d[i + 2];
    h[0][r >> 2]++;
    h[1][g >> 2]++;
    h[2][b >> 2]++;
    h[3][
      Math.min(255, Math.round(0.2126 * r + 0.7152 * g + 0.0722 * b)) >> 2
    ]++;
  }
  return h;
}

/** Studio skin: RGB and luma histogram of the processed picture. */
function Histogram({ data }: { data: number[][] | null }) {
  const W = 256;
  const H = 64;
  const paths = useMemo(() => {
    if (!data) return null;
    // Scale against the second highest bin, so one clipped spike does not flatten everything.
    const all = data
      .slice(0, 3)
      .flat()
      .sort((a, b) => b - a);
    const top = Math.max(1, all[2] ?? 1);
    const pts = (bins: number[]) =>
      bins.map(
        (v, i) =>
          `${((i + 0.5) / BINS) * W},${H - Math.min(1, v / top) * (H - 2)}`,
      );
    const area = (bins: number[]) =>
      `M0,${H} L${pts(bins).join(" L")} L${W},${H} Z`;
    return {
      r: area(data[0]),
      g: area(data[1]),
      b: area(data[2]),
      l: `M${pts(data[3]).join(" L")}`,
    };
  }, [data]);
  return (
    <div className="histo">
      <div className="histo-head">
        <span>Histogram</span>
        <span className="mono muted">after</span>
      </div>
      <svg
        viewBox={`0 0 ${W} ${H}`}
        preserveAspectRatio="none"
        className="histo-svg"
        aria-label="Histogram of the adjusted picture"
      >
        <g className="histo-grid">
          {[64, 128, 192].map((x) => (
            <line key={x} x1={x} x2={x} y1={0} y2={H} />
          ))}
        </g>
        {paths && (
          <>
            <path d={paths.r} className="h-r" />
            <path d={paths.g} className="h-g" />
            <path d={paths.b} className="h-b" />
            <path d={paths.l} className="h-l" />
          </>
        )}
      </svg>
      <div className="histo-scale mono muted">
        <span>0</span>
        <span>64</span>
        <span>128</span>
        <span>192</span>
        <span>255</span>
      </div>
    </div>
  );
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
  const [hist, setHist] = useState<number[][] | null>(null);
  const ref = refById(picture);
  const skin = useStore().settings.theme.skin;
  const studio = skin === "studio";

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
    const id = requestAnimationFrame(() => {
      if (!after.current) return;
      const out = render(after.current, pixels, look);
      if (studio) setHist(histogram(out));
    });
    return () => cancelAnimationFrame(id);
  }, [pixels, look, studio]);

  const drag = (clientX: number) => {
    const r = box.current?.getBoundingClientRect();
    if (!r) return;
    setSplit(Math.min(100, Math.max(0, ((clientX - r.left) / r.width) * 100)));
  };

  return (
    <div className="preview">
      <div className="preview-pick">
        {REF_PICTURES.map((r) => (
          <button
            key={r.id}
            type="button"
            className={`pill ${r.id === ref?.id ? "active" : ""}`}
            onClick={() => onPicture(r.id)}
          >
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
          <div className="preview-empty muted">
            Add a picture named ref-something.jpg to the assets folder to
            preview here.
          </div>
        ) : (
          <>
            <canvas
              ref={after}
              width={PREVIEW_W}
              height={PREVIEW_H}
              className="preview-canvas"
            />
            <canvas
              ref={before}
              width={PREVIEW_W}
              height={PREVIEW_H}
              className="preview-canvas"
              style={{ clipPath: `inset(0 ${100 - split}% 0 0)` }}
            />
            {skin === "terrain" && look && (
              <span className="hud mono" aria-hidden="true">
                <span>GAM {look.tone.gamma.toFixed(2)}</span>
                <span>BRT {Math.round(look.tone.brightness)}</span>
                <span>CON {Math.round(look.tone.contrast)}</span>
                <span>VIB {Math.round(look.color.vibrance)}</span>
              </span>
            )}
            <span className="badge left">Before</span>
            <span className="badge right">After</span>
            <div className="split-line" style={{ left: `${split}%` }}>
              <span className="split-handle">
                <svg
                  width="18"
                  height="18"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                >
                  <path d="M9 6l-5 6 5 6 M15 6l5 6-5 6" />
                </svg>
              </span>
            </div>
          </>
        )}
      </div>
      {skin === "calibration" && <StepWedge look={look} />}
      {studio && <Histogram data={hist} />}
    </div>
  );
}

/** Small live thumbnail of a look on a reference picture. */
export function Thumb({
  look,
  picture,
  width = 200,
  height = 112,
}: {
  look: Look;
  picture: string;
  width?: number;
  height?: number;
}) {
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
  return (
    <canvas ref={canvas} width={width} height={height} className="thumb" />
  );
}
