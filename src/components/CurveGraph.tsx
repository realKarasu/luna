import type { Tone } from "../types";
import { channelValue, clampTone } from "../lib/math";

const W = 340;
const H = 150;
const PAD = 8;

function path(t: Tone, ch: 0 | 1 | 2): string {
  const pts: string[] = [];
  for (let i = 0; i <= 64; i++) {
    const x = i / 64;
    const y = channelValue(x, t, ch);
    pts.push(`${(PAD + x * (W - 2 * PAD)).toFixed(1)},${(H - PAD - y * (H - 2 * PAD)).toFixed(1)}`);
  }
  return `M${pts.join(" L")}`;
}

/** Output curve per channel with the dashed identity line. */
export function CurveGraph({ tone }: { tone: Tone }) {
  const t = clampTone(tone);
  const same = t.red === t.green && t.green === t.blue;
  return (
    <svg className="curve" viewBox={`0 0 ${W} ${H}`} role="img" aria-label="Output curve">
      {[0.25, 0.5, 0.75].map((g) => (
        <g key={g} className="grid">
          <line x1={PAD + g * (W - 2 * PAD)} x2={PAD + g * (W - 2 * PAD)} y1={PAD} y2={H - PAD} />
          <line y1={PAD + g * (H - 2 * PAD)} y2={PAD + g * (H - 2 * PAD)} x1={PAD} x2={W - PAD} />
        </g>
      ))}
      <rect x={PAD} y={PAD} width={W - 2 * PAD} height={H - 2 * PAD} className="frame" rx="4" />
      <line className="identity" x1={PAD} y1={H - PAD} x2={W - PAD} y2={PAD} />
      {same ? (
        <path d={path(t, 0)} className="curve-line all" />
      ) : (
        <>
          <path d={path(t, 0)} className="curve-line r" />
          <path d={path(t, 1)} className="curve-line g" />
          <path d={path(t, 2)} className="curve-line b" />
        </>
      )}
    </svg>
  );
}
