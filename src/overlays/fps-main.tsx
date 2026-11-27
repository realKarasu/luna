import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import type { FpsReading } from "../types";
import { isTauri } from "../lib/backend";
import "./overlay.css";

const show = (v: number, digits = 0) => (v > 0 ? v.toFixed(digits) : "--");
const part = (v: number | null | undefined, unit: string, digits = 0) => (v == null ? `--${unit}` : `${v.toFixed(digits)}${unit}`);

// Heights of the window in the units of engine.rs: 200 wide, 76 for the counter,
// 42 more for the graph and 42 more for the details.
const BASE = 76;
const EXTRA = 42;

const GRAPH_W = 182;
const GRAPH_H = 30;

const DEMO: FpsReading = {
  state: "running",
  fps: 143.6,
  ms: 6.96,
  low: 97.2,
  exe: "game.exe",
  gaps: Array.from({ length: 90 }, (_, i) => 6.9 + Math.sin(i / 5) * 0.7 + (i % 31 === 17 ? 5.5 : 0) + (i % 7) * 0.12),
  sys: { cpu: 38, gpu: 97, gpuTemp: 64, vram: 9.4, ram: 18.2 },
};

/** Frame times as a line, newest on the right. A steady game draws a flat line in the middle. */
function Graph({ gaps }: { gaps: number[] }) {
  const sorted = [...gaps].sort((a, b) => a - b);
  const median = sorted[Math.floor(sorted.length / 2)] ?? 0;
  const top = Math.max(median * 2, sorted[sorted.length - 1] ?? 0, 1);
  const step = GRAPH_W / Math.max(gaps.length - 1, 1);
  const points = gaps.map((g, i) => `${(i * step).toFixed(1)},${(GRAPH_H - 1 - (g / top) * (GRAPH_H - 2)).toFixed(1)}`).join(" ");
  return (
    <svg className="fps-graph" viewBox={`0 0 ${GRAPH_W} ${GRAPH_H}`} preserveAspectRatio="none" aria-hidden="true">
      <line className="fps-graph-base" x1="0" x2={GRAPH_W} y1={GRAPH_H - 0.5} y2={GRAPH_H - 0.5} />
      {gaps.length > 1 && <polyline className="fps-graph-line" points={points} />}
    </svg>
  );
}

/** How tall the window is, in the units above. Luna resizes it when the settings change. */
function useUnits(): number {
  const read = () => (isTauri && window.innerWidth > 0 ? (window.innerHeight / window.innerWidth) * 200 : BASE + EXTRA * 2);
  const [units, setUnits] = useState(read);
  useEffect(() => {
    const on = () => setUnits(read());
    window.addEventListener("resize", on);
    return () => window.removeEventListener("resize", on);
  }, []);
  return units;
}

function FpsOverlay() {
  const [r, setR] = useState<FpsReading | null>(null);
  const units = useUnits();
  useEffect(() => {
    if (!isTauri) {
      setR(DEMO);
      return;
    }
    let off: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => listen<FpsReading>("fps", (e) => setR(e.payload)).then((u) => (off = u)));
    return () => off?.();
  }, []);

  // One extra band is the graph or the details: the details come with their numbers.
  const bands = Math.round((units - BASE) / EXTRA);
  const sys = bands >= 1 ? (r?.sys ?? null) : null;
  const graph = bands >= 2 || (bands === 1 && !sys);
  const running = !r || r.state === "running";

  return (
    <div className="fps">
      {running ? (
        <>
          <div className="fps-main">
            <span className="fps-value">{show(r?.fps ?? 0)}</span>
            <span className="fps-unit">FPS</span>
          </div>
          <div className="fps-meta">
            <span>{show(r?.ms ?? 0, 1)} ms</span>
            <span>1% {show(r?.low ?? 0)}</span>
          </div>
        </>
      ) : (
        <div className="fps-note">{r.state === "needsAdmin" ? "FPS needs Luna as administrator" : "FPS counter unavailable"}</div>
      )}
      {graph && <Graph gaps={running ? (r?.gaps ?? []) : []} />}
      {sys && (
        <div className="fps-sys">
          <div className="fps-sys-row">
            <span>CPU {part(sys.cpu, "%")}</span>
            <span>GPU {part(sys.gpu, "%")}</span>
            {sys.gpuTemp != null && <span>{sys.gpuTemp.toFixed(0)}°C</span>}
          </div>
          <div className="fps-sys-row">
            <span>VRAM {part(sys.vram, " GB", 1)}</span>
            <span>RAM {part(sys.ram, " GB", 1)}</span>
          </div>
        </div>
      )}
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<FpsOverlay />);
