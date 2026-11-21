// Talks to the Rust side inside the app. In a plain browser (no Tauri) it runs a
// small demo backend so the interface can still be explored.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { AppState, Bind, CleanItem, CleanReport, CleanResult, DisplayMode, Fit, GamesStatus, MonitorInfo, Page, PotatoStatus, Settings, SharePayload, TweakStatus, UpdateStatus, WindowInfo } from "../types";
import { defaultSettings } from "./defaults";
import { decodeShare, encodeShare } from "./sharecode";

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

type Unlisten = () => void;

function on<T>(event: string, cb: (payload: T) => void): Unlisten {
  if (!isTauri) {
    const handler = (e: Event) => cb((e as CustomEvent<T>).detail);
    window.addEventListener(`luna:${event}`, handler);
    return () => window.removeEventListener(`luna:${event}`, handler);
  }
  let off: (() => void) | null = null;
  let cancelled = false;
  listen<T>(event, (e) => cb(e.payload)).then((u) => {
    if (cancelled) u();
    else off = u;
  });
  return () => {
    cancelled = true;
    off?.();
  };
}

// ---------------- demo backend ----------------

const DEMO_KEY = "luna-demo-settings";
const DEMO_MONITORS: MonitorInfo[] = [
  {
    id: "\\\\.\\DISPLAY1",
    label: "Display 1",
    x: 0,
    y: 0,
    width: 2560,
    height: 1440,
    primary: true,
    gpu: "AMD Radeon RX 7800 XT",
    vendor: "amd",
    vibrance: "AMD saturation",
  },
  {
    id: "\\\\.\\DISPLAY2",
    label: "Display 2",
    x: 2560,
    y: 0,
    width: 1920,
    height: 1080,
    primary: false,
    gpu: "AMD Radeon RX 7800 XT",
    vendor: "amd",
    vibrance: "AMD saturation",
  },
];
const demoTweaks: Record<string, boolean> = {};
const DEMO_MODES: Record<string, DisplayMode[]> = {
  big: [2560, 1920, 1600, 1280].flatMap((w) => {
    const h = Math.round((w * 9) / 16);
    return [165, 144, 120, 60].map((hz) => ({ width: w, height: h, hz }));
  }),
  small: [1920, 1600, 1280].flatMap((w) => {
    const h = Math.round((w * 9) / 16);
    return [240, 144, 60].map((hz) => ({ width: w, height: h, hz }));
  }),
};
const POTATO_OFF: PotatoStatus = { on: false, manual: false, game: null, tweaks: [], slowed: 0, boosted: null };
let demoPotato: PotatoStatus = POTATO_OFF;
let demoWindows: WindowInfo[] = [
  { id: 101, title: "Ashfall Frontier", exe: "ashfall.exe", width: 2560, height: 1440, borderless: false, framed: true },
  { id: 102, title: "Voice chat", exe: "voicechat.exe", width: 1280, height: 760, borderless: false, framed: true },
  { id: 103, title: "notes.txt - Notepad", exe: "notepad.exe", width: 900, height: 620, borderless: false, framed: true },
];
const demoSet = (id: number, borderless: boolean) => {
  demoWindows = demoWindows.map((w) => (w.id === id ? { ...w, borderless, framed: !borderless } : w));
  return demoWindows;
};
const MB = 1024 * 1024;
const demoItem = (id: string, group: CleanItem["group"], title: string, desc: string, mb: number, files: number, extra: Partial<CleanItem> = {}): CleanItem => ({
  id,
  group,
  title,
  desc,
  bytes: Math.round(mb * MB),
  files,
  suggested: true,
  needsAdmin: false,
  open: null,
  ...extra,
});
let demoClean: CleanItem[] = [
  demoItem("temp", "system", "Temporary files", "Files programs leave in your temp folder. Only those older than a day go.", 4605, 33793),
  demoItem("windowsTemp", "system", "Windows temporary files", "The temp folder Windows and installers use. Only files older than a day go.", 0, 0, { needsAdmin: true }),
  demoItem("crashReports", "system", "Crash dumps and error reports", "Copies of memory and reports kept after a program or game crashed.", 139, 10),
  demoItem("updateCache", "system", "Windows Update downloads", "Update files already installed. Windows downloads again whatever it still needs.", 189, 18, { needsAdmin: true }),
  demoItem("thumbnails", "system", "Thumbnail cache", "Small previews of pictures and videos. Windows draws them again, folders open a bit slower at first.", 73, 30, { suggested: false }),
  demoItem("recycleBin", "system", "Recycle Bin", "Everything you deleted. It cannot be brought back after this.", 312, 41, { suggested: false }),
  demoItem("chrome", "browsers", "Chrome", "Cached pages and pictures. History, passwords and cookies stay.", 23, 131),
  demoItem("firefox", "browsers", "Firefox", "Cached pages and pictures. History, passwords and cookies stay.", 1054, 6605, { open: "Firefox" }),
  demoItem("discord", "apps", "Discord", "Cached pictures, emojis and videos from your chats.", 455, 4764, { open: "Discord" }),
  demoItem("steam", "apps", "Steam", "Store and library page cache, old logs and crash dumps. Games and saves stay.", 523, 4818),
  demoItem("spotify", "apps", "Spotify", "Songs kept to play them faster. Downloads for offline listening stay.", 599, 1016),
  demoItem("shaders", "apps", "Graphics card shader cache", "Shaders games build on your graphics card. They are rebuilt on the next launch, which can stutter for a few minutes. Worth it after a driver update.", 529, 889, { suggested: false }),
  demoItem("devCaches", "dev", "Package caches", "Downloads kept by npm, Yarn, pip, NuGet, Cargo and Go. Projects keep working, the next install downloads again.", 2898, 24639, { suggested: false }),
];
let demoBackground: string | null = null;

function demoLoad(): Settings {
  try {
    const raw = localStorage.getItem(DEMO_KEY);
    if (raw) return JSON.parse(raw) as Settings;
  } catch {
    /* ignore */
  }
  const s = defaultSettings();
  s.selectedMonitor = DEMO_MONITORS[0].id;
  return s;
}

function demoSave(s: Settings) {
  try {
    localStorage.setItem(DEMO_KEY, JSON.stringify(s));
  } catch {
    /* storage full or blocked */
  }
}

const TWEAK_IDS = [
  "power",
  "gameMode",
  "gameDvr",
  "flipModel",
  "mouseAccel",
  "stickyKeys",
  "notifications",
  "backgroundApps",
  "transparency",
  "animations",
];

function demoTweakStatus(): TweakStatus[] {
  return TWEAK_IDS.map((id) => ({ id, on: !!demoTweaks[id], available: true, backedUp: !!demoTweaks[id] }));
}

const DOM_KEYS: Record<string, [number, string]> = {
  Escape: [0x1b, "Esc"],
  Space: [0x20, "Space"],
  Tab: [0x09, "Tab"],
  CapsLock: [0x14, "Caps Lock"],
  ShiftLeft: [0xa0, "Shift"],
  ShiftRight: [0xa1, "Right Shift"],
  ControlLeft: [0xa2, "Ctrl"],
  ControlRight: [0xa3, "Right Ctrl"],
  AltLeft: [0xa4, "Alt"],
  AltRight: [0xa5, "Right Alt"],
};

function demoRecord(): Promise<void> {
  const finish = (b: Bind | null) => {
    window.removeEventListener("keydown", onKey, true);
    window.removeEventListener("mousedown", onMouse, true);
    window.dispatchEvent(new CustomEvent("luna:bind-recorded", { detail: b }));
  };
  const onKey = (e: KeyboardEvent) => {
    e.preventDefault();
    const fixed = DOM_KEYS[e.code];
    if (fixed) return finish(fixed[0] === 0x1b ? null : { vk: fixed[0], label: fixed[1] });
    const label = /^F\d+$/.test(e.key) ? e.key : e.key.length === 1 ? e.key.toUpperCase() : e.code;
    finish({ vk: e.keyCode, label });
  };
  const onMouse = (e: MouseEvent) => {
    const map = [1, 4, 2, 5, 6];
    // Left clicks are clicks in the page itself (same rule as the app).
    if (e.button === 0) return;
    e.preventDefault();
    finish({ vk: map[e.button] ?? 1, label: `Mouse ${[1, 3, 2, 4, 5][e.button] ?? 1}` });
  };
  // Start listening after the click that started recording has finished.
  setTimeout(() => {
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("mousedown", onMouse, true);
  }, 150);
  return Promise.resolve();
}

// ---------------- the API ----------------

export const IDLE_UPDATE: UpdateStatus = {
  state: "idle",
  version: null,
  notes: null,
  progress: 0,
  error: null,
  checkedAt: null,
};

export const api = {
  async getState(): Promise<AppState> {
    if (isTauri) return invoke<AppState>("get_state");
    return {
      settings: demoLoad(),
      monitors: DEMO_MONITORS,
      driverVibrance: true,
      magnifier: true,
      gammaStrength: {},
      version: "0.7.0",
      dataDir: "%APPDATA%\\Luna",
      autostart: false,
      update: IDLE_UPDATE,
      platform: "windows",
      optimizer: true,
      note: null,
      startPage: null,
    };
  },
  async setSettings(s: Settings): Promise<Record<string, number>> {
    if (isTauri) return invoke("set_settings", { settings: s });
    demoSave(s);
    return {};
  },
  async resetDisplay(current: Settings): Promise<Settings> {
    if (isTauri) return invoke<Settings>("reset_display");
    const s: Settings = {
      ...current,
      tones: {},
      activeGamma: {},
      color: { hue: 0, saturation: 100, vibrance: 100 },
      activeColor: "",
      holo: { ...current.holo, mode: "off" },
    };
    demoSave(s);
    return s;
  },
  startBindRecord: (): Promise<void> => (isTauri ? invoke("start_bind_record") : demoRecord()),
  cancelBindRecord: (): Promise<void> => (isTauri ? invoke("cancel_bind_record") : Promise.resolve()),
  onBindRecorded: (cb: (b: Bind | null) => void) => on<Bind | null>("bind-recorded", cb),
  onSettingsChanged: (cb: (s: Settings) => void) => on<Settings>("settings-changed", cb),
  onMonitorsChanged: (cb: (m: MonitorInfo[]) => void) => on<MonitorInfo[]>("monitors-changed", cb),

  async optimizerStatus(): Promise<TweakStatus[]> {
    if (isTauri) return invoke("optimizer_status");
    return demoTweakStatus();
  },
  async optimizerSet(id: string, value: boolean): Promise<TweakStatus[]> {
    if (isTauri) return invoke("optimizer_set", { id, on: value });
    demoTweaks[id] = value;
    return demoTweakStatus();
  },
  async optimizerAll(): Promise<{ errors: string[]; status: TweakStatus[] }> {
    if (isTauri) return invoke("optimizer_all");
    TWEAK_IDS.forEach((id) => (demoTweaks[id] = true));
    return { errors: [], status: demoTweakStatus() };
  },
  async optimizerUndoAll(): Promise<{ errors: string[]; status: TweakStatus[] }> {
    if (isTauri) return invoke("optimizer_undo_all");
    TWEAK_IDS.forEach((id) => (demoTweaks[id] = false));
    return { errors: [], status: demoTweakStatus() };
  },
  async cleanScan(): Promise<CleanReport> {
    if (isTauri) return invoke("clean_scan");
    await new Promise((r) => setTimeout(r, 900));
    return { items: demoClean.filter((i) => i.bytes > 0 || i.needsAdmin), admin: false };
  },
  async cleanRun(ids: string[]): Promise<CleanResult> {
    if (isTauri) return invoke("clean_run", { ids });
    let freedBytes = 0;
    let files = 0;
    demoClean = demoClean.map((i) => {
      if (!ids.includes(i.id) || i.needsAdmin) return i;
      const kept = i.open ? Math.round(i.files * 0.1) : 0;
      freedBytes += i.bytes * (1 - kept / Math.max(1, i.files));
      files += i.files - kept;
      return { ...i, bytes: i.bytes * (kept / Math.max(1, i.files)), files: kept };
    });
    return { freedBytes: Math.round(freedBytes), files, skipped: 0 };
  },
  /** Starts Luna again as administrator, then closes this copy. */
  async restartAdmin(page: Page): Promise<void> {
    if (isTauri) return invoke("restart_admin", { page });
  },
  async borderlessWindows(): Promise<WindowInfo[]> {
    if (isTauri) return invoke("borderless_windows");
    return demoWindows;
  },
  async borderlessApply(id: number, fit: Fit): Promise<WindowInfo[]> {
    if (isTauri) return invoke("borderless_apply", { id, fit });
    return demoSet(id, true);
  },
  async borderlessRestore(id: number): Promise<WindowInfo[]> {
    if (isTauri) return invoke("borderless_restore", { id });
    return demoSet(id, false);
  },
  async displayModes(monitor: string): Promise<DisplayMode[]> {
    if (isTauri) return invoke("display_modes", { monitor });
    return monitor.endsWith("2") ? DEMO_MODES.small : DEMO_MODES.big;
  },
  async gamesStatus(): Promise<GamesStatus> {
    if (isTauri) return invoke("games_status");
    return { active: null, potato: demoPotato, fps: null };
  },
  async potatoSet(on: boolean): Promise<GamesStatus> {
    if (isTauri) return invoke("potato_set", { on });
    demoPotato = on ? { on: true, manual: true, game: null, tweaks: ["power", "gameMode", "transparency"], slowed: 14, boosted: "ashfall.exe" } : POTATO_OFF;
    return { active: null, potato: demoPotato, fps: null };
  },
  /** A game with a profile started or closed, or Potato Mode changed. */
  onGameChanged: (cb: (s: GamesStatus) => void) => on<GamesStatus>("game-changed", cb),
  onGameError: (cb: (e: string) => void) => on<string>("game-error", cb),
  /** The FPS counter: "running", "needsAdmin", "error" or "off". */
  onFpsState: (cb: (state: string) => void) => on<string>("fps-state", cb),
  /** The borderless key was used in game; carries the reason when it did nothing. */
  onBorderlessChanged: (cb: (error: string | null) => void) => on<string | null>("borderless-changed", cb),
  async shareEncode(p: SharePayload): Promise<string> {
    if (isTauri) return invoke("share_encode", { payload: p });
    return encodeShare(p);
  },
  async shareDecode(code: string): Promise<SharePayload> {
    if (isTauri) return invoke("share_decode", { code });
    return decodeShare(code);
  },
  async setAutostart(value: boolean): Promise<boolean> {
    if (isTauri) return invoke("set_autostart", { on: value });
    return value;
  },
  async getBackground(): Promise<string | null> {
    if (isTauri) return invoke("get_background");
    return demoBackground;
  },
  async setBackground(data: string | null): Promise<void> {
    if (isTauri) return invoke("set_background", { data });
    demoBackground = data;
  },
  async openDataDir(): Promise<void> {
    if (isTauri) return invoke("open_data_dir");
  },
  async quit(): Promise<void> {
    if (isTauri) return invoke("quit_app");
  },
  async updateCheck(): Promise<UpdateStatus> {
    if (isTauri) return invoke("update_check");
    return { ...IDLE_UPDATE, state: "latest", checkedAt: Math.floor(Date.now() / 1000) };
  },
  async updateInstall(): Promise<void> {
    if (isTauri) return invoke("update_install");
  },
  onUpdateStatus: (cb: (u: UpdateStatus) => void) => on<UpdateStatus>("update-status", cb),
};

export async function windowAction(action: "minimize" | "maximize" | "close") {
  if (!isTauri) return;
  const { getCurrentWindow } = await import("@tauri-apps/api/window");
  const w = getCurrentWindow();
  if (action === "minimize") await w.minimize();
  else if (action === "maximize") await w.toggleMaximize();
  else await w.close();
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return "Something went wrong.";
}
