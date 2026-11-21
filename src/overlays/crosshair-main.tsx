import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Crosshair } from "../types";
import { api, isTauri } from "../lib/backend";
import { DEFAULT_CROSSHAIR } from "../lib/defaults";
import { CrosshairSvg } from "../components/CrosshairSvg";
import "./overlay.css";

const SIZE = 256;

function CrosshairOverlay() {
  const [c, setC] = useState<Crosshair>(DEFAULT_CROSSHAIR);
  useEffect(() => {
    api
      .getState()
      .then((s) => s.settings?.crosshair?.current && setC({ ...DEFAULT_CROSSHAIR, ...s.settings.crosshair.current }))
      .catch(() => undefined);
    if (!isTauri) return;
    let off: (() => void) | undefined;
    import("@tauri-apps/api/event").then(({ listen }) =>
      listen<Crosshair>("crosshair", (e) => setC({ ...DEFAULT_CROSSHAIR, ...e.payload })).then((u) => (off = u)),
    );
    return () => off?.();
  }, []);
  return (
    <div className="overlay-center">
      {/* The window is SIZE physical pixels; the drawing fills it whatever the Windows scale. */}
      <CrosshairSvg c={c} size={SIZE} fluid />
    </div>
  );
}

createRoot(document.getElementById("root")!).render(<CrosshairOverlay />);
