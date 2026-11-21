import { useEffect, useState } from "react";
import type { Page, Theme } from "./types";
import { StoreProvider, useStore } from "./store";
import { api, isTauri } from "./lib/backend";
import { SkyCanvas } from "./components/SkyCanvas";
import { Sidebar, StatusBar, TitleBar, Toasts } from "./components/Chrome";
import { DisplayPage } from "./pages/Display";
import { PresetsPage } from "./pages/Presets";
import { CrosshairPage } from "./pages/CrosshairPage";
import { OptimizerPage } from "./pages/Optimizer";
import { BindsPage } from "./pages/Binds";
import { ThemePage } from "./pages/ThemePage";
import { SettingsPage } from "./pages/SettingsPage";

const FONTS: Record<Theme["font"], string> = {
  hanken: '"Hanken Grotesk", "Segoe UI", system-ui, sans-serif',
  segoe: '"Segoe UI Variable Text", "Segoe UI", system-ui, sans-serif',
  bahnschrift: 'Bahnschrift, "Segoe UI", system-ui, sans-serif',
  mono: '"JetBrains Mono", Consolas, monospace',
};
const RADII: Record<Theme["radius"], [string, string]> = {
  sharp: ["6px", "4px"],
  soft: ["18px", "10px"],
  round: ["26px", "16px"],
};

function mix(hex: string, with_: number[], amount: number): string {
  const n = parseInt(hex.replace("#", ""), 16);
  if (!Number.isFinite(n)) return hex;
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v, i) => Math.round(v + (with_[i] - v) * amount));
  return `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

function applyTheme(t: Theme) {
  const r = document.documentElement.style;
  const accent = /^#[0-9a-f]{6}$/i.test(t.accent) ? t.accent : "#a86dff";
  const e = Math.min(100, Math.max(0, t.effect)) / 100;
  r.setProperty("--accent", accent);
  r.setProperty("--accent-2", mix(accent, [255, 255, 255], 0.45));
  r.setProperty("--accent-tint", `${accent}1c`);
  r.setProperty("--accent-soft", `${accent}33`);
  r.setProperty("--blur", `${Math.round(4 + e * 22)}px`);
  r.setProperty("--glow", `${(e * 0.6).toFixed(2)}`);
  r.setProperty("--glass", `rgba(255,255,255,${(0.035 + e * 0.04).toFixed(3)})`);
  r.setProperty("--font", FONTS[t.font] ?? FONTS.hanken);
  r.setProperty("--r-panel", RADII[t.radius]?.[0] ?? "18px");
  r.setProperty("--r-ctl", RADII[t.radius]?.[1] ?? "10px");
}

function Shell() {
  const { settings } = useStore();
  const [page, setPage] = useState<Page>("display");
  const [background, setBackground] = useState<string | null>(null);

  useEffect(() => applyTheme(settings.theme), [settings.theme]);
  useEffect(() => {
    api.getBackground().then(setBackground).catch(() => setBackground(null));
  }, []);
  useEffect(() => {
    document.documentElement.classList.toggle("in-app", isTauri);
  }, []);

  const custom = settings.theme.background === "custom" && background;

  return (
    <div className="app">
      <div className="backdrop">
        {custom ? <div className="custom-bg" style={{ backgroundImage: `url(${background})` }} /> : <SkyCanvas accent={settings.theme.accent} effect={settings.theme.effect} />}
        <div className="backdrop-shade" />
      </div>
      <TitleBar />
      <div className="body">
        <Sidebar page={page} onPage={setPage} />
        <main className="content" key={page}>
          {page === "display" && <DisplayPage />}
          {page === "presets" && <PresetsPage />}
          {page === "crosshair" && <CrosshairPage />}
          {page === "optimizer" && <OptimizerPage />}
          {page === "binds" && <BindsPage />}
          {page === "theme" && <ThemePage background={background} onBackground={setBackground} />}
          {page === "settings" && <SettingsPage />}
        </main>
      </div>
      <StatusBar />
      <Toasts />
    </div>
  );
}

export default function App() {
  return (
    <StoreProvider>
      <Shell />
    </StoreProvider>
  );
}
