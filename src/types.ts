// Shapes shared with the Rust side (serde camelCase).

export interface Tone {
  gamma: number;
  brightness: number;
  contrast: number;
  red: number;
  green: number;
  blue: number;
}

export interface ColorParams {
  hue: number;
  saturation: number;
  vibrance: number;
}

export interface Bind {
  vk: number;
  label: string;
}

export interface Binds {
  zoom: Bind | null;
  nvg: Bind | null;
  tuning: Bind | null;
  color: (Bind | null)[];
  wheel: Bind | null;
}

export interface GammaPreset {
  id: string;
  name: string;
  builtin: boolean;
  tone: Tone;
}

export interface ColorPreset {
  id: string;
  name: string;
  builtin: boolean;
  color: ColorParams;
}

export type CrosshairDesign =
  | "classic"
  | "dotcross"
  | "tee"
  | "circle"
  | "circledot"
  | "dot"
  | "chevron"
  | "xcross"
  | "brackets"
  | "ringcross";

export interface Crosshair {
  design: CrosshairDesign;
  color: string;
  ring: number;
  length: number;
  gap: number;
  thickness: number;
  dot: number;
  outline: number;
  opacity: number;
}

export interface CrosshairSettings {
  enabled: boolean;
  monitor: string;
  hotkey: boolean;
  current: Crosshair;
  slots: (Crosshair | null)[];
  slotNames: string[];
}

export interface Theme {
  background: "sky" | "custom";
  accent: string;
  effect: number;
  font: "hanken" | "segoe" | "bahnschrift" | "mono";
  radius: "sharp" | "soft" | "round";
  skin: Skin;
}

export type Skin = "nebula" | "observatoire" | "calibration" | "terrain" | "studio";

export interface Settings {
  version: number;
  tuningOn: boolean;
  selectedMonitor: string;
  tones: Record<string, Tone>;
  color: ColorParams;
  activeGamma: Record<string, string>;
  activeColor: string;
  profile: "general" | "rust";
  nvg: { mode: "green" | "blue"; monitor: string };
  holo: { mode: "off" | "greenTrueBlack" | "redBlackHolo"; strength: number };
  zoom: { factor: number; monitor: string };
  binds: Binds;
  colorKeys: string[];
  wheelPresets: string[];
  gammaPresets: GammaPreset[];
  colorPresets: ColorPreset[];
  presetsSeeded: boolean;
  crosshair: CrosshairSettings;
  theme: Theme;
  app: { startWithWindows: boolean; startMinimized: boolean; closeToTray: boolean; autoUpdate: boolean };
}

export interface MonitorInfo {
  id: string;
  label: string;
  x: number;
  y: number;
  width: number;
  height: number;
  primary: boolean;
  /** The graphics card driving this screen, empty when Windows did not say. */
  gpu: string;
  vendor: GpuVendor;
  /** What gives this screen its vibrance: "NVIDIA digital vibrance", "AMD saturation" or "Color matrix". */
  vibrance: string;
}

export type GpuVendor = "nvidia" | "amd" | "intel" | "generic";

export interface AppState {
  settings: Settings;
  monitors: MonitorInfo[];
  /** At least one screen gets its vibrance from the graphics driver. */
  driverVibrance: boolean;
  magnifier: boolean;
  gammaStrength: Record<string, number>;
  version: string;
  dataDir: string;
  autostart: boolean;
  update: UpdateStatus;
  /** "windows", "macos" or "linux". */
  platform: string;
  /** The Windows Optimizer page exists on this system. */
  optimizer: boolean;
  /** Something the user must know or do on this system, or null. */
  note: string | null;
}

export interface UpdateStatus {
  state: "idle" | "checking" | "downloading" | "ready" | "latest" | "error";
  version: string | null;
  notes: string | null;
  /** 0 to 1 while downloading. */
  progress: number;
  error: string | null;
  /** Unix seconds of the last finished check. */
  checkedAt: number | null;
}

export interface SharePayload {
  kind: "gamma" | "color" | "crosshair";
  name: string;
  values: number[];
}

export interface TweakStatus {
  id: string;
  on: boolean;
  available: boolean;
  backedUp: boolean;
}

export interface CleanResult {
  freedBytes: number;
  files: number;
  skipped: number;
}

export type Page = "display" | "presets" | "crosshair" | "zoom" | "optimizer" | "binds" | "theme" | "settings";
