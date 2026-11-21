import { useEffect, useState } from "react";
import type { Page, Theme } from "./types";
import { StoreProvider, useStore } from "./store";
import { api, isTauri } from "./lib/backend";
import { SkyCanvas } from "./components/SkyCanvas";
import { Sidebar, StatusBar, TitleBar, Toasts } from "./components/Chrome";
import { DisplayPage } from "./pages/Display";
import { PresetsPage } from "./pages/Presets";
import { CrosshairPage } from "./pages/CrosshairPage";
import { ZoomPage } from "./pages/ZoomPage";
import { OptimizerPage } from "./pages/Optimizer";
import { BindsPage } from "./pages/Binds";
import { ThemePage } from "./pages/ThemePage";
import { SettingsPage } from "./pages/SettingsPage";
import { skinInfo } from "./lib/skins";

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
  const c = [(n >> 16) & 255, (n >> 8) & 255, n & 255].map((v, i) =>
    Math.round(v + (with_[i] - v) * amount),
  );
  return `#${c.map((v) => v.toString(16).padStart(2, "0")).join("")}`;
}

function applyTheme(t: Theme) {
  const root = document.documentElement;
  const r = root.style;
  const skin = skinInfo(t.skin);
  const accent = /^#[0-9a-f]{6}$/i.test(t.accent) ? t.accent : skin.accent;
  root.dataset.skin = skin.id;
  r.setProperty("--accent", accent);
  // On the light skin the second accent is a darker shade, so it still reads as ink.
  r.setProperty(
    "--accent-2",
    skin.light
      ? mix(accent, [0, 0, 0], 0.2)
      : mix(accent, [255, 255, 255], 0.45),
  );
  r.setProperty("--accent-tint", `${accent}1c`);
  r.setProperty("--accent-soft", `${accent}33`);
  // Font, corners and glass belong to Nebula; the other skins set their own in skins.css.
  const own = ["--blur", "--glow", "--glass", "--font", "--r-panel", "--r-ctl"];
  if (skin.id !== "nebula") {
    own.forEach((k) => r.removeProperty(k));
    return;
  }
  const e = Math.min(100, Math.max(0, t.effect)) / 100;
  r.setProperty("--blur", `${Math.round(4 + e * 22)}px`);
  r.setProperty("--glow", `${(e * 0.6).toFixed(2)}`);
  r.setProperty(
    "--glass",
    `rgba(255,255,255,${(0.035 + e * 0.04).toFixed(3)})`,
  );
  r.setProperty("--font", FONTS[t.font] ?? FONTS.hanken);
  r.setProperty("--r-panel", RADII[t.radius]?.[0] ?? "18px");
  r.setProperty("--r-ctl", RADII[t.radius]?.[1] ?? "10px");
}

function Shell() {
  const { settings, info } = useStore();
  const [page, setPage] = useState<Page>("display");
  const [background, setBackground] = useState<string | null>(null);

  useEffect(() => applyTheme(settings.theme), [settings.theme]);
  useEffect(() => {
    api
      .getBackground()
      .then(setBackground)
      .catch(() => setBackground(null));
  }, []);
  useEffect(() => {
    document.documentElement.classList.toggle("in-app", isTauri);
  }, []);

  const custom = settings.theme.background === "custom" && background;
  const nebula = skinInfo(settings.theme.skin).id === "nebula";

  return (
    <div className="app">
      {nebula && (
        <div className="backdrop">
          {custom ? (
            <div
              className="custom-bg"
              style={{ backgroundImage: `url(${background})` }}
            />
          ) : (
            <SkyCanvas art={settings.theme.background === "nebula" ? "nebula" : "sky"} />
          )}
          <div className="backdrop-shade" />
        </div>
      )}
      <TitleBar />
      <div className="body">
        <Sidebar page={page} onPage={setPage} />
        <main className="content" key={page}>
          {info.note && <p className="note warn platform-note">{info.note}</p>}
          {page === "display" && <DisplayPage />}
          {page === "presets" && <PresetsPage />}
          {page === "crosshair" && <CrosshairPage />}
          {page === "zoom" && info.platform === "windows" && <ZoomPage />}
          {page === "optimizer" && info.optimizer && <OptimizerPage />}
          {page === "binds" && <BindsPage />}
          {page === "theme" && (
            <ThemePage background={background} onBackground={setBackground} />
          )}
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
