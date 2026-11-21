// Port of src-tauri/src/gamma.rs and color.rs. Keep the formulas identical:
// the preview must show exactly what the real output does.
import type { ColorParams, Tone } from "../types";

export const NEUTRAL_TONE: Tone = { gamma: 1, brightness: 0, contrast: 100, red: 100, green: 100, blue: 100 };
export const NEUTRAL_COLOR: ColorParams = { hue: 0, saturation: 100, vibrance: 100 };

const clamp = (v: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, v));
const fin = (v: number, d: number) => (Number.isFinite(v) ? v : d);

export function clampTone(t: Tone): Tone {
  return {
    gamma: clamp(fin(t.gamma, 1), 0.6, 3),
    brightness: clamp(fin(t.brightness, 0), -20, 20),
    contrast: clamp(fin(t.contrast, 100), 70, 130),
    red: clamp(fin(t.red, 100), 20, 180),
    green: clamp(fin(t.green, 100), 20, 180),
    blue: clamp(fin(t.blue, 100), 20, 180),
  };
}

export function clampColor(p: ColorParams): ColorParams {
  return {
    hue: clamp(fin(p.hue, 0), -180, 180),
    saturation: clamp(fin(p.saturation, 100), 0, 350),
    vibrance: clamp(fin(p.vibrance, 100), 0, 250),
  };
}

/** gamma.rs `channel_value`: gamma, contrast around mid grey, brightness, channel gain, clamp. */
export function channelValue(x: number, t: Tone, ch: 0 | 1 | 2): number {
  x = clamp(x, 0, 1);
  let v = Math.pow(x, 1 / t.gamma);
  v = (v - 0.5) * (t.contrast / 100) + 0.5;
  v += t.brightness / 200;
  v *= (ch === 0 ? t.red : ch === 1 ? t.green : t.blue) / 100;
  return clamp(v, 0, 1);
}

/** 3 x 256 lookup tables in 0..255, from the same 16-bit ramp Windows receives. */
export function buildLut(tone: Tone): Uint8ClampedArray[] {
  const t = clampTone(tone);
  return [0, 1, 2].map((ch) => {
    const lut = new Uint8ClampedArray(256);
    for (let i = 0; i < 256; i++) {
      const word = Math.round(channelValue(i / 255, t, ch as 0 | 1 | 2) * 65535);
      lut[i] = Math.round(word / 257);
    }
    return lut;
  });
}

// ---------- colour matrices (column vectors, out = M * [r g b a 1]) ----------

export type Mat5 = number[][];
const LR = 0.2126;
const LG = 0.7152;
const LB = 0.0722;

export function identity(): Mat5 {
  return [0, 1, 2, 3, 4].map((i) => [0, 1, 2, 3, 4].map((j) => (i === j ? 1 : 0)));
}

export function mul(a: Mat5, b: Mat5): Mat5 {
  const out = identity();
  for (let i = 0; i < 5; i++)
    for (let j = 0; j < 5; j++) {
      let s = 0;
      for (let k = 0; k < 5; k++) s += a[i][k] * b[k][j];
      out[i][j] = s;
    }
  return out;
}

export function saturationMatrix(s: number): Mat5 {
  const m = identity();
  m[0][0] = LR + (1 - LR) * s;
  m[0][1] = LG - LG * s;
  m[0][2] = LB - LB * s;
  m[1][0] = LR - LR * s;
  m[1][1] = LG + (1 - LG) * s;
  m[1][2] = LB - LB * s;
  m[2][0] = LR - LR * s;
  m[2][1] = LG - LG * s;
  m[2][2] = LB + (1 - LB) * s;
  return m;
}

export function hueMatrix(deg: number): Mat5 {
  const r = (deg * Math.PI) / 180;
  const s = Math.sin(r);
  const c = Math.cos(r);
  const m = identity();
  m[0][0] = LR + c * (1 - LR) - s * LR;
  m[0][1] = LG - c * LG - s * LG;
  m[0][2] = LB - c * LB + s * (1 - LB);
  m[1][0] = LR - c * LR + s * 0.143;
  m[1][1] = LG + c * (1 - LG) + s * 0.14;
  m[1][2] = LB - c * LB - s * 0.283;
  m[2][0] = LR - c * LR - s * (1 - LR);
  m[2][1] = LG - c * LG + s * LG;
  m[2][2] = LB + c * (1 - LB) + s * LB;
  return m;
}

export type HoloMode = "off" | "greenTrueBlack" | "redBlackHolo";

export function holoMatrix(mode: HoloMode, strength: number): Mat5 {
  const k = clamp(strength, 0, 1);
  const m = identity();
  if (mode === "off") return m;
  const ch = mode === "greenTrueBlack" ? 1 : 0;
  for (let j = 0; j < 3; j++) m[ch][j] = j === ch ? 1 - k : k / 2;
  return m;
}

/** Order: holo on the game's colours, then hue, then saturation (x vibrance factor). */
export function colorMatrix(p: ColorParams, vibFactor: number, holo: HoloMode = "off", holoStrength = 0): Mat5 {
  const c = clampColor(p);
  const s = (c.saturation / 100) * Math.max(0, vibFactor);
  return mul(saturationMatrix(s), mul(hueMatrix(c.hue), holoMatrix(holo, holoStrength)));
}

/**
 * The preview shows vibrance as the saturation factor (vibrance / 100). That is
 * exactly the output on screens without driver vibrance; on NVIDIA and AMD the
 * driver covers what it can (usually up to 200 %) and the matrix the rest.
 */
export function previewMatrix(p: ColorParams, holo: HoloMode = "off", holoStrength = 0): Mat5 {
  const c = clampColor(p);
  return colorMatrix(c, c.vibrance / 100, holo, holoStrength);
}

export function isIdentity(m: Mat5): boolean {
  const id = identity();
  return m.every((row, i) => row.every((v, j) => Math.abs(v - id[i][j]) < 1e-6));
}

/**
 * Applies the real pipeline to pixels: the colour matrix (composited by Windows)
 * first, then the gamma ramp (applied by the graphics card on the way out).
 */
export function processPixels(src: Uint8ClampedArray, dst: Uint8ClampedArray, m: Mat5, lut: Uint8ClampedArray[]) {
  const [lr, lg, lb] = lut;
  const identityMatrix = isIdentity(m);
  const m00 = m[0][0], m01 = m[0][1], m02 = m[0][2], m04 = m[0][4] * 255 + m[0][3] * 255;
  const m10 = m[1][0], m11 = m[1][1], m12 = m[1][2], m14 = m[1][4] * 255 + m[1][3] * 255;
  const m20 = m[2][0], m21 = m[2][1], m22 = m[2][2], m24 = m[2][4] * 255 + m[2][3] * 255;
  for (let i = 0; i < src.length; i += 4) {
    let r = src[i], g = src[i + 1], b = src[i + 2];
    if (!identityMatrix) {
      const nr = m00 * r + m01 * g + m02 * b + m04;
      const ng = m10 * r + m11 * g + m12 * b + m14;
      const nb = m20 * r + m21 * g + m22 * b + m24;
      r = nr < 0 ? 0 : nr > 255 ? 255 : Math.round(nr);
      g = ng < 0 ? 0 : ng > 255 ? 255 : Math.round(ng);
      b = nb < 0 ? 0 : nb > 255 ? 255 : Math.round(nb);
    }
    dst[i] = lr[r];
    dst[i + 1] = lg[g];
    dst[i + 2] = lb[b];
    dst[i + 3] = src[i + 3];
  }
}

export const NVG_TONES: Record<"green" | "blue", Tone> = {
  green: { gamma: 3, brightness: 0, contrast: 100, red: 35, green: 100, blue: 35 },
  blue: { gamma: 3, brightness: 0, contrast: 100, red: 35, green: 70, blue: 100 },
};
