import type { ColorParams, ColorPreset, GammaPreset, Settings, SharePayload, Tone, Crosshair, CrosshairDesign } from "../types";
import { clampColor, clampTone, NEUTRAL_TONE } from "./math";
import { CROSSHAIR_DESIGNS, DEFAULT_CROSSHAIR } from "./defaults";

export function toneOf(s: Settings, monitor: string): Tone {
  return s.tones[monitor] ?? NEUTRAL_TONE;
}

export function setTone(s: Settings, monitor: string, tone: Tone, presetId = ""): Settings {
  return {
    ...s,
    tones: { ...s.tones, [monitor]: clampTone(tone) },
    activeGamma: { ...s.activeGamma, [monitor]: presetId },
  };
}

export function setColor(s: Settings, color: ColorParams, presetId = ""): Settings {
  return { ...s, color: clampColor(color), activeColor: presetId };
}

export const applyGamma = (s: Settings, p: GammaPreset, monitor: string) => setTone(s, monitor, p.tone, p.id);
export const applyColor = (s: Settings, p: ColorPreset) => setColor(s, p.color, p.id);

const sameNum = (a: number, b: number) => Math.abs(a - b) < 0.005;
export const sameTone = (a: Tone, b: Tone) =>
  sameNum(a.gamma, b.gamma) &&
  sameNum(a.brightness, b.brightness) &&
  sameNum(a.contrast, b.contrast) &&
  sameNum(a.red, b.red) &&
  sameNum(a.green, b.green) &&
  sameNum(a.blue, b.blue);
export const sameColor = (a: ColorParams, b: ColorParams) =>
  sameNum(a.hue, b.hue) && sameNum(a.saturation, b.saturation) && sameNum(a.vibrance, b.vibrance);

// ---------- share payloads ----------

export function gammaPayload(name: string, t: Tone): SharePayload {
  return { kind: "gamma", name, values: [t.gamma, t.brightness, t.contrast, t.red, t.green, t.blue] };
}

export function colorPayload(name: string, c: ColorParams): SharePayload {
  return { kind: "color", name, values: [c.hue, c.saturation, c.vibrance] };
}

export function crosshairPayload(name: string, c: Crosshair): SharePayload {
  const idx = Math.max(0, CROSSHAIR_DESIGNS.findIndex((d) => d.id === c.design));
  const rgb = parseInt(c.color.replace("#", ""), 16) || 0;
  return {
    kind: "crosshair",
    name,
    values: [idx, (rgb >> 16) & 255, (rgb >> 8) & 255, rgb & 255, c.ring, c.length, c.gap, c.thickness, c.dot, c.outline, c.opacity],
  };
}

export function toneFromPayload(p: SharePayload): Tone {
  const [gamma, brightness, contrast, red, green, blue] = p.values;
  return clampTone({
    gamma: gamma ?? 1,
    brightness: brightness ?? 0,
    contrast: contrast ?? 100,
    red: red ?? 100,
    green: green ?? 100,
    blue: blue ?? 100,
  });
}

export function colorFromPayload(p: SharePayload): ColorParams {
  const [hue, saturation, vibrance] = p.values;
  return clampColor({ hue: hue ?? 0, saturation: saturation ?? 100, vibrance: vibrance ?? 100 });
}

const clampN = (v: number | undefined, lo: number, hi: number, d: number) =>
  v === undefined || !Number.isFinite(v) ? d : Math.min(hi, Math.max(lo, v));

export function crosshairFromPayload(p: SharePayload): Crosshair {
  const v = p.values;
  const design: CrosshairDesign = CROSSHAIR_DESIGNS[Math.round(clampN(v[0], 0, CROSSHAIR_DESIGNS.length - 1, 0))].id;
  const hex = (n: number | undefined) => Math.round(clampN(n, 0, 255, 255)).toString(16).padStart(2, "0");
  return {
    design,
    color: `#${hex(v[1])}${hex(v[2])}${hex(v[3])}`,
    ring: clampN(v[4], 2, 40, DEFAULT_CROSSHAIR.ring),
    length: clampN(v[5], 0, 40, DEFAULT_CROSSHAIR.length),
    gap: clampN(v[6], 0, 30, DEFAULT_CROSSHAIR.gap),
    thickness: clampN(v[7], 0.5, 10, DEFAULT_CROSSHAIR.thickness),
    dot: clampN(v[8], 0, 10, DEFAULT_CROSSHAIR.dot),
    outline: clampN(v[9], 0, 4, DEFAULT_CROSSHAIR.outline),
    opacity: clampN(v[10], 5, 100, DEFAULT_CROSSHAIR.opacity),
  };
}
