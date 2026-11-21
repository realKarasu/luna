import { useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { isTauri } from "../lib/backend";
import "./overlay.css";

interface WheelItem {
  id: string;
  name: string;
  kind: string;
}
interface WheelState {
  visible: boolean;
  items: WheelItem[];
  selected: number;
}

const SIZE = 560;
const C = SIZE / 2;
const R_OUT = 250;
const R_IN = 92;

function polar(r: number, deg: number) {
  const a = ((deg - 90) * Math.PI) / 180;
  return [C + r * Math.cos(a), C + r * Math.sin(a)];
}

function slice(i: number, n: number) {
  const span = 360 / n;
  const a0 = i * span - span / 2 + 1.2;
  const a1 = i * span + span / 2 - 1.2;
  const [x0, y0] = polar(R_OUT, a0);
  const [x1, y1] = polar(R_OUT, a1);
  const [x2, y2] = polar(R_IN, a1);
  const [x3, y3] = polar(R_IN, a0);
  const large = a1 - a0 > 180 ? 1 : 0;
  return `M${x0},${y0} A${R_OUT},${R_OUT} 0 ${large} 1 ${x1},${y1} L${x2},${y2} A${R_IN},${R_IN} 0 ${large} 0 ${x3},${y3} Z`;
}

/** Item 0 sits at the top, then clockwise (same as engine.rs `wheel_index`). */
function Wheel() {
  const [st, setSt] = useState<WheelState>({ visible: false, items: [], selected: -1 });
  const [accent, setAccent] = useState("#a86dff");

  useEffect(() => {
    if (!isTauri) {
      // Browser preview: show a sample wheel.
      setSt({
        visible: true,
        selected: 1,
        items: ["Ultra Vivid", "Neon", "Pastel", "Noir", "Cartoon", "Hue 16", "Hue 32", "Desert Glow"].map((name, i) => ({ id: String(i), name, kind: "color" })),
      });
      return;
    }
    let off: (() => void) | undefined;
    import("@tauri-apps/api/core").then(({ invoke }) =>
      invoke<{ settings: { theme: { accent: string } } }>("get_state").then((s) => setAccent(s.settings.theme.accent)).catch(() => undefined),
    );
    import("@tauri-apps/api/event").then(({ listen }) => listen<WheelState>("wheel-state", (e) => setSt(e.payload)).then((u) => (off = u)));
    return () => off?.();
  }, []);

  if (!st.visible || st.items.length === 0) return null;
  const n = st.items.length;
  const sel = st.items[st.selected];
  return (
    <svg className="wheel" width="100%" height="100%" viewBox={`0 0 ${SIZE} ${SIZE}`} style={{ ["--accent" as string]: accent }}>
      <defs>
        <radialGradient id="wheel-bg" cx="50%" cy="50%" r="50%">
          <stop offset="0.3" stopColor="#140f22" stopOpacity="0.92" />
          <stop offset="1" stopColor="#0d0a16" stopOpacity="0.88" />
        </radialGradient>
      </defs>
      {st.items.map((it, i) => {
        const [tx, ty] = polar((R_OUT + R_IN) / 2, (360 / n) * i);
        const active = i === st.selected;
        return (
          <g key={it.id} className={`wheel-item ${active ? "active" : ""}`}>
            <path d={slice(i, n)} fill={active ? "var(--accent)" : "url(#wheel-bg)"} fillOpacity={active ? 0.85 : 1} />
            <text x={tx} y={ty - 4} textAnchor="middle" className="wheel-name">
              {it.name.length > 14 ? `${it.name.slice(0, 13)}.` : it.name}
            </text>
            <text x={tx} y={ty + 14} textAnchor="middle" className="wheel-kind">
              {it.kind}
            </text>
          </g>
        );
      })}
      <circle cx={C} cy={C} r={R_IN - 10} className="wheel-hub" />
      <text x={C} y={C - 4} textAnchor="middle" className="wheel-hub-title">
        {sel ? sel.name : "Luna"}
      </text>
      <text x={C} y={C + 16} textAnchor="middle" className="wheel-hub-hint">
        {sel ? "release to apply" : "point to pick"}
      </text>
    </svg>
  );
}

createRoot(document.getElementById("root")!).render(<Wheel />);
