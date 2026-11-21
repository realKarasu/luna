import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import type { FpsReading } from "../types";
import { isTauri } from "../lib/backend";
import "./overlay.css";

const show = (v: number, digits = 0) => (v > 0 ? v.toFixed(digits) : "--");

function FpsOverlay() {
  const [r, setR] = useState<FpsReading | null>(null);
  useEffect(() => {
    if (!isTauri) {
      setR({ state: "running", fps: 143.6, ms: 6.96, low: 97.2, exe: "game.exe" });
      return;
    }
    let off: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) => listen<FpsReading>("fps", (e) => setR(e.payload)).then((u) => (off = u)));
    return () => off?.();
  }, []);

  if (r && r.state !== "running") {
    return (
      <div className="fps">
        <div className="fps-note">{r.state === "needsAdmin" ? "FPS needs Luna as administrator" : "FPS counter unavailable"}</div>
      </div>
    );
  }
  return (
    <div className="fps">
      <div className="fps-main">
        <span className="fps-value">{show(r?.fps ?? 0)}</span>
        <span className="fps-unit">FPS</span>
      </div>
      <div className="fps-meta">
        <span>{show(r?.ms ?? 0, 1)} ms</span>
        <span>1% {show(r?.low ?? 0)}</span>
      </div>
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<FpsOverlay />);
