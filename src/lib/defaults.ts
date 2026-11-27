import type { ColorPreset, Crosshair, CrosshairDesign, GameProfile, GammaPreset, Settings, Tone } from "../types";

const tone = (gamma: number, brightness: number, contrast: number, red = 100, green = 100, blue = 100): Tone => ({
  gamma,
  brightness,
  contrast,
  red,
  green,
  blue,
});

/** The 15 built-in gamma presets. Ids are stable so deleted ones can come back. */
export const BUILTIN_GAMMA: GammaPreset[] = [
  { id: "g-afterglow", name: "Afterglow", builtin: true, tone: tone(1.35, 2, 104, 108, 100, 90) },
  { id: "g-lifted", name: "Lifted", builtin: true, tone: tone(1.5, 0, 100) },
  { id: "g-shadow-lift", name: "Shadow Lift", builtin: true, tone: tone(1.9, 3, 96) },
  { id: "g-deep-lift", name: "Deep Lift", builtin: true, tone: tone(2.4, 5, 92) },
  { id: "g-green-nvg", name: "Green NVG", builtin: true, tone: tone(3, 0, 100, 35, 100, 35) },
  { id: "g-blue-nvg", name: "Blue NVG", builtin: true, tone: tone(3, 0, 100, 35, 70, 100) },
  { id: "g-moonlight", name: "Moonlight", builtin: true, tone: tone(1.6, 0, 102, 92, 98, 112) },
  { id: "g-night-owl", name: "Night Owl", builtin: true, tone: tone(2.1, 4, 98, 96, 100, 104) },
  { id: "g-crisp", name: "Crisp", builtin: true, tone: tone(1.15, 0, 112) },
  { id: "g-soft-dusk", name: "Soft Dusk", builtin: true, tone: tone(1.25, -2, 94, 104, 100, 96) },
  { id: "g-ember", name: "Ember", builtin: true, tone: tone(1.4, 0, 106, 116, 98, 82) },
  { id: "g-arctic", name: "Arctic", builtin: true, tone: tone(1.3, 0, 104, 90, 100, 115) },
  { id: "g-full-bright", name: "Full Bright", builtin: true, tone: tone(2.8, 8, 90) },
  { id: "g-low-glare", name: "Low Glare", builtin: true, tone: tone(0.85, -6, 96) },
  { id: "g-ink", name: "Ink Shadows", builtin: true, tone: tone(0.75, 0, 110) },
];

/** The 12 built-in colour presets. */
export const BUILTIN_COLOR: ColorPreset[] = [
  { id: "c-ultra-vivid", name: "Ultra Vivid", builtin: true, color: { hue: 0, saturation: 190, vibrance: 170 } },
  { id: "c-neon", name: "Neon", builtin: true, color: { hue: 0, saturation: 240, vibrance: 200 } },
  { id: "c-pastel", name: "Pastel", builtin: true, color: { hue: 0, saturation: 60, vibrance: 90 } },
  { id: "c-noir", name: "Noir", builtin: true, color: { hue: 0, saturation: 0, vibrance: 100 } },
  { id: "c-cartoon", name: "Cartoon", builtin: true, color: { hue: 0, saturation: 220, vibrance: 150 } },
  { id: "c-hue-16", name: "Hue 16", builtin: true, color: { hue: 16, saturation: 110, vibrance: 100 } },
  { id: "c-hue-32", name: "Hue 32", builtin: true, color: { hue: 32, saturation: 110, vibrance: 100 } },
  { id: "c-desert-glow", name: "Desert Glow", builtin: true, color: { hue: -8, saturation: 130, vibrance: 140 } },
  { id: "c-rich-color", name: "Rich Color", builtin: true, color: { hue: 0, saturation: 145, vibrance: 130 } },
  { id: "c-cool-shift", name: "Cool Shift", builtin: true, color: { hue: -20, saturation: 115, vibrance: 105 } },
  { id: "c-faded-film", name: "Faded Film", builtin: true, color: { hue: 6, saturation: 75, vibrance: 85 } },
  { id: "c-max-pop", name: "Max Pop", builtin: true, color: { hue: 0, saturation: 300, vibrance: 250 } },
];

export const CROSSHAIR_DESIGNS: { id: CrosshairDesign; name: string }[] = [
  { id: "classic", name: "Classic" },
  { id: "dotcross", name: "Cross + dot" },
  { id: "tee", name: "Tee" },
  { id: "circle", name: "Ring" },
  { id: "circledot", name: "Ring + dot" },
  { id: "dot", name: "Dot" },
  { id: "chevron", name: "Chevron" },
  { id: "xcross", name: "X" },
  { id: "brackets", name: "Brackets" },
  { id: "ringcross", name: "Scope" },
];

export const CROSSHAIR_COLORS: { hex: string; name: string }[] = [
  { hex: "#3dff8b", name: "Green" },
  { hex: "#ff3b5c", name: "Red" },
  { hex: "#ffe14d", name: "Yellow" },
  { hex: "#38e8ff", name: "Cyan" },
  { hex: "#ff5cf0", name: "Magenta" },
  { hex: "#ffffff", name: "White" },
  { hex: "#b98cff", name: "Lilac" },
  { hex: "#ff9a3d", name: "Orange" },
];

export const DEFAULT_CROSSHAIR: Crosshair = {
  design: "classic",
  color: "#3dff8b",
  ring: 10,
  length: 8,
  gap: 4,
  thickness: 2,
  dot: 2,
  outline: 1,
  opacity: 100,
};

export const ACCENTS = ["#a86dff", "#6d8bff", "#38d6ff", "#3dffb0", "#ffcf5a", "#ff7b8a", "#ff6dd6", "#e8e4f0"];

export function defaultSettings(): Settings {
  return {
    version: 1,
    tuningOn: true,
    selectedMonitor: "",
    tones: {},
    color: { hue: 0, saturation: 100, vibrance: 100 },
    activeGamma: {},
    activeColor: "",
    profile: "general",
    nvg: { mode: "green", monitor: "" },
    holo: { mode: "off", strength: 85 },
    zoom: { factor: 2.5, monitor: "" },
    borderless: { fit: { mode: "stay", monitor: "", width: 1920, height: 1080 }, games: [] },
    binds: { zoom: null, nvg: null, tuning: null, color: [null, null, null, null], wheel: { vk: 0x76, label: "F7" }, borderless: null, fps: null, potato: null },
    games: [],
    fps: { enabled: false, corner: "topLeft", monitor: "", graph: false, details: false },
    colorKeys: ["", "", "", ""],
    wheelPresets: [],
    gammaPresets: [],
    colorPresets: [],
    presetsSeeded: false,
    crosshair: {
      enabled: false,
      monitor: "",
      hotkey: false,
      current: { ...DEFAULT_CROSSHAIR },
      slots: Array(8).fill(null),
      slotNames: Array(8).fill(""),
    },
    theme: { background: "sky", accent: "#a86dff", effect: 60, font: "hanken", radius: "soft", skin: "nebula" },
    app: { startWithWindows: false, startMinimized: false, closeToTray: true, autoUpdate: true },
  };
}

/** Fills in anything missing (first run, older files) and seeds the built-in presets once. */
export const DEFAULT_PROFILE: GameProfile = {
  exe: "",
  name: "",
  enabled: true,
  monitor: "",
  gamma: "",
  color: "",
  crosshair: "",
  mode: null,
  otherModes: [],
  potato: false,
};

export function normalize(s: Settings): Settings {
  const d = defaultSettings();
  const out: Settings = {
    ...d,
    ...s,
    nvg: { ...d.nvg, ...s.nvg },
    holo: { ...d.holo, ...s.holo },
    zoom: { ...d.zoom, ...s.zoom },
    borderless: {
      fit: { ...d.borderless.fit, ...s.borderless?.fit },
      games: Array.isArray(s.borderless?.games) ? s.borderless.games : [],
    },
    games: Array.isArray(s.games) ? s.games.map((g) => ({ ...DEFAULT_PROFILE, ...g })) : [],
    fps: { ...d.fps, ...s.fps },
    binds: { ...d.binds, ...s.binds },
    crosshair: { ...d.crosshair, ...s.crosshair, current: { ...DEFAULT_CROSSHAIR, ...s.crosshair?.current } },
    theme: { ...d.theme, ...s.theme },
    app: { ...d.app, ...s.app },
  };
  const pad = <T,>(arr: T[] | undefined, n: number, fill: T): T[] => {
    const a = Array.isArray(arr) ? arr.slice(0, n) : [];
    while (a.length < n) a.push(fill);
    return a;
  };
  out.binds.color = pad(out.binds.color, 4, null);
  out.colorKeys = pad(out.colorKeys, 4, "");
  out.crosshair.slots = pad(out.crosshair.slots, 8, null);
  out.crosshair.slotNames = pad(out.crosshair.slotNames, 8, "");
  if (!out.presetsSeeded) {
    out.gammaPresets = [...BUILTIN_GAMMA, ...(out.gammaPresets ?? []).filter((p) => !p.builtin)];
    out.colorPresets = [...BUILTIN_COLOR, ...(out.colorPresets ?? []).filter((p) => !p.builtin)];
    out.presetsSeeded = true;
    if (out.wheelPresets.length === 0) out.wheelPresets = BUILTIN_COLOR.slice(0, 8).map((p) => p.id);
    if (out.colorKeys.every((k) => !k)) out.colorKeys = ["c-ultra-vivid", "c-rich-color", "c-noir", "c-hue-16"];
  }
  return out;
}

export function newId(prefix: string): string {
  return `${prefix}-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`;
}
