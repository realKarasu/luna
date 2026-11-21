import type { Page } from "../types";
import { useStore } from "../store";
import { api, errorText, isTauri, windowAction } from "../lib/backend";
import { vendorLabel, vibranceSummary } from "../lib/gpu";
import { Icon, MoonLogo } from "./Icon";

export function TitleBar() {
  const { settings, info, toast } = useStore();
  const ready = info.update.state === "ready";
  const restart = () => api.updateInstall().catch((e) => toast(errorText(e), "bad"));
  return (
    <header className="titlebar" data-tauri-drag-region>
      <div className="brand" data-tauri-drag-region>
        <MoonLogo size={24} />
        <span className="brand-name" data-tauri-drag-region>
          Luna
        </span>
        <span className="chip version" data-tauri-drag-region>
          v{info.version}
        </span>
      </div>
      <div className="titlebar-right">
        <span className={`status ${settings.tuningOn ? "on" : "off"}`}>
          <span className="dot" />
          {settings.tuningOn ? "Tuning active" : "Tuning paused"}
        </span>
        {ready && (
          <button
            type="button"
            className="chip accent update-chip"
            title={`Version ${info.update.version} is downloaded. It also installs by itself when Luna quits.`}
            onClick={restart}
          >
            <Icon name="download" size={13} /> Restart to update
          </button>
        )}
        {!isTauri && <span className="chip warn">Browser preview</span>}
        <button type="button" className="win-btn" aria-label="Minimize" onClick={() => windowAction("minimize")}>
          <Icon name="minimize" size={16} />
        </button>
        <button type="button" className="win-btn" aria-label="Maximize" onClick={() => windowAction("maximize")}>
          <Icon name="maximize" size={13} />
        </button>
        <button type="button" className="win-btn close" aria-label="Close" onClick={() => windowAction("close")}>
          <Icon name="close" size={16} />
        </button>
      </div>
    </header>
  );
}

const MENU: { id: Page; label: string; icon: string }[] = [
  { id: "display", label: "Display", icon: "display" },
  { id: "presets", label: "Presets", icon: "presets" },
  { id: "crosshair", label: "Crosshair", icon: "crosshair" },
  { id: "zoom", label: "Zoom", icon: "zoom" },
  { id: "borderless", label: "Borderless", icon: "borderless" },
  { id: "optimizer", label: "Optimizer", icon: "optimizer" },
];
const GENERAL: { id: Page; label: string; icon: string }[] = [
  { id: "binds", label: "Binds", icon: "binds" },
  { id: "theme", label: "Theme", icon: "theme" },
  { id: "settings", label: "Settings", icon: "settings" },
];

export function Sidebar({ page, onPage }: { page: Page; onPage: (p: Page) => void }) {
  const { settings, monitors, update, info } = useStore();
  const mon = monitors.find((m) => m.id === settings.selectedMonitor) ?? monitors[0];
  const gammaId = mon ? settings.activeGamma[mon.id] : "";
  const gamma = settings.gammaPresets.find((p) => p.id === gammaId);
  const color = settings.colorPresets.find((p) => p.id === settings.activeColor);
  const presetLabel = [gamma?.name, color?.name].filter(Boolean).join(" + ") || (mon && settings.tones[mon.id] ? "Custom" : "Neutral");

  const item = (it: { id: Page; label: string; icon: string }) => (
    <button key={it.id} type="button" className={`nav-item ${page === it.id ? "active" : ""}`} onClick={() => onPage(it.id)}>
      <Icon name={it.icon} />
      <span>{it.label}</span>
    </button>
  );

  return (
    <aside className="sidebar">
      <nav className="nav-groups" aria-label="Sections">
        <div className="nav-group">
          <div className="nav-title">Menu</div>
          {MENU.filter((m) => (m.id !== "optimizer" || info.optimizer) && ((m.id !== "zoom" && m.id !== "borderless") || info.platform === "windows")).map(item)}
        </div>
        <div className="nav-group">
          <div className="nav-title">General</div>
          {GENERAL.map(item)}
        </div>
      </nav>
      <div className="side-spacer" />
      <button
        type="button"
        className={`tuning-card ${settings.tuningOn ? "on" : ""}`}
        onClick={() => update((s) => ({ ...s, tuningOn: !s.tuningOn }))}
        title="Turn display tuning on or off"
      >
        <div className="tuning-card-top">
          <span className="dot" />
          <span>{settings.tuningOn ? "Tuning on" : "Tuning off"}</span>
        </div>
        <div className="tuning-card-line">
          <span className="muted">Preset</span>
          <span className="ellipsis">{presetLabel}</span>
        </div>
        <div className="tuning-card-line">
          <span className="muted">Display</span>
          <span>{mon?.label ?? "None"}</span>
        </div>
      </button>
    </aside>
  );
}

export function StatusBar() {
  const { settings, monitors } = useStore();
  const screens = monitors.length === 1 ? "1 screen" : `${monitors.length} screens`;
  const cards = [...new Set(monitors.map((m) => vendorLabel(m.vendor)))].join(" + ");
  return (
    <footer className="statusbar">
      <span>{screens}</span>
      {cards && (
        <>
          <span className="sep" />
          <span>GPU: {cards}</span>
        </>
      )}
      <span className="sep" />
      <span>{vibranceSummary(monitors)}</span>
      <span className="sep" />
      <span>Profile: {settings.profile === "rust" ? "Rust" : "General"}</span>
      <span className="statusbar-right muted">Settings saved on this PC only</span>
    </footer>
  );
}

export function Toasts() {
  const { toasts } = useStore();
  return (
    <div className="toasts" role="status" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast ${t.tone}`}>
          {t.text}
        </div>
      ))}
    </div>
  );
}
