import type { ReactElement } from "react";
import type { Crosshair } from "../types";

/** Draws a crosshair centred in a size x size box. Used by the editor and the overlay. */
export function CrosshairSvg({ c, size = 256, fluid = false }: { c: Crosshair; size?: number; fluid?: boolean }) {
  const cx = size / 2;
  const cy = size / 2;
  const th = Math.max(0.5, c.thickness);
  const gap = Math.max(0, c.gap);
  const len = Math.max(0, c.length);
  const ring = Math.max(1, c.ring);
  const dot = Math.max(0, c.dot);
  const ol = Math.max(0, c.outline);

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const parts: ReactElement<any>[] = [];
  const line = (x1: number, y1: number, x2: number, y2: number, k: string) =>
    parts.push(<line key={k} x1={x1} y1={y1} x2={x2} y2={y2} />);
  const arms = (top = true) => {
    if (top) line(cx, cy - gap, cx, cy - gap - len, "t");
    line(cx, cy + gap, cx, cy + gap + len, "b");
    line(cx - gap, cy, cx - gap - len, cy, "l");
    line(cx + gap, cy, cx + gap + len, cy, "r");
  };
  const circle = (r: number, k: string) => parts.push(<circle key={k} cx={cx} cy={cy} r={r} fill="none" />);
  const centreDot = (k = "d") => dot > 0 && parts.push(<circle key={k} cx={cx} cy={cy} r={dot} className="fill" />);

  switch (c.design) {
    case "classic":
      arms();
      break;
    case "dotcross":
      arms();
      centreDot();
      break;
    case "tee":
      arms(false);
      centreDot();
      break;
    case "circle":
      circle(ring, "c");
      break;
    case "circledot":
      circle(ring, "c");
      centreDot();
      break;
    case "dot":
      parts.push(<circle key="d" cx={cx} cy={cy} r={Math.max(1, dot || th)} className="fill" />);
      break;
    case "chevron": {
      const s = Math.max(2, len);
      parts.push(<polyline key="v" points={`${cx - s},${cy + gap + s} ${cx},${cy + gap} ${cx + s},${cy + gap + s}`} fill="none" />);
      centreDot();
      break;
    }
    case "xcross": {
      const g = gap / Math.SQRT2;
      const l = (gap + len) / Math.SQRT2;
      line(cx - g, cy - g, cx - l, cy - l, "a");
      line(cx + g, cy - g, cx + l, cy - l, "b");
      line(cx - g, cy + g, cx - l, cy + l, "c");
      line(cx + g, cy + g, cx + l, cy + l, "d");
      centreDot();
      break;
    }
    case "brackets": {
      const r = Math.max(gap + 2, ring);
      const s = Math.max(2, len * 0.6);
      parts.push(<polyline key="l" points={`${cx - r + s},${cy - r} ${cx - r},${cy - r} ${cx - r},${cy + r} ${cx - r + s},${cy + r}`} fill="none" />);
      parts.push(<polyline key="r" points={`${cx + r - s},${cy - r} ${cx + r},${cy - r} ${cx + r},${cy + r} ${cx + r - s},${cy + r}`} fill="none" />);
      centreDot();
      break;
    }
    case "ringcross": {
      circle(ring, "c");
      line(cx, cy - ring - len * 0.6, cx, cy - gap, "t");
      line(cx, cy + ring + len * 0.6, cx, cy + gap, "b");
      line(cx - ring - len * 0.6, cy, cx - gap, cy, "l");
      line(cx + ring + len * 0.6, cy, cx + gap, cy, "r");
      centreDot();
      break;
    }
  }

  return (
    <svg width={fluid ? "100%" : size} height={fluid ? "100%" : size} viewBox={`0 0 ${size} ${size}`} style={{ opacity: Math.min(1, Math.max(0.05, c.opacity / 100)) }} aria-hidden="true">
      {ol > 0 && (
        <g stroke="#000" strokeWidth={th + ol * 2} strokeLinecap="square" fill="none" className="outline">
          {parts.map((p) => (
            <g key={`o-${p.key}`} {...(p.props.className === "fill" ? { fill: "#000", strokeWidth: ol * 2 } : {})}>
              {p}
            </g>
          ))}
        </g>
      )}
      <g stroke={c.color} strokeWidth={th} strokeLinecap="square" fill="none">
        {parts.map((p) => (
          <g key={`m-${p.key}`} {...(p.props.className === "fill" ? { fill: c.color, stroke: "none" } : {})}>
            {p}
          </g>
        ))}
      </g>
    </svg>
  );
}
