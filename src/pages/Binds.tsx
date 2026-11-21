import type { ReactNode } from "react";
import { useStore } from "../store";
import { Chip, Panel, Select } from "../components/ui";
import { BindButton, slotId, useBindRecorder, type Slot } from "../components/BindButton";

export function BindsPage() {
  const { settings, info, update } = useStore();
  const rec = useBindRecorder();

  const presetOptions = [
    { value: "", label: "Nothing" },
    ...settings.colorPresets.map((p) => ({ value: p.id, label: `Color: ${p.name}` })),
    ...settings.gammaPresets.map((p) => ({ value: p.id, label: `Gamma: ${p.name}` })),
  ];

  const bindButton = (slot: Slot) => <BindButton slot={slot} rec={rec} />;

  const line = (slot: Slot, title: string, desc: string, extra?: ReactNode) => (
    <div className="bind-row" key={slotId(slot)}>
      <div className="row-text">
        <div className="row-title">
          {title} {extra}
        </div>
        <div className="muted small">{desc}</div>
      </div>
      {bindButton(slot)}
    </div>
  );

  const wheelAll = [...settings.colorPresets, ...settings.gammaPresets];
  const toggleWheel = (id: string) =>
    update((s) => {
      const has = s.wheelPresets.includes(id);
      if (!has && s.wheelPresets.length >= 8) return s;
      return { ...s, wheelPresets: has ? s.wheelPresets.filter((w) => w !== id) : [...s.wheelPresets, id] };
    });

  return (
    <div className="page">
      <Panel title="Binds" subtitle="Click a key box, then press the key or mouse button (1 to 5) you want. Esc cancels. Keys work in game, even when Luna is in the background.">
        <div className="bind-list">
          {line({ key: "zoom" }, "Ultra Zoom", "Hold to zoom into the middle of the screen.")}
          {line({ key: "nvg" }, "Night vision", "Hold for green or blue night vision.", settings.profile !== "rust" ? <Chip tone="warn">Rust profile only</Chip> : null)}
          {line({ key: "tuning" }, "Tuning on/off", "Press to switch every adjustment on or off.")}
          {line({ key: "wheel" }, "Look wheel", "Hold, point the mouse at a preset, release to apply it.")}
          {info.platform === "windows" && line({ key: "borderless" }, "Borderless window", "Press in game to take the frame off the window in front. Press again to put it back.")}
          {info.platform === "windows" && line({ key: "fps" }, "FPS counter", "Press to show or hide the frame counter in the corner of the screen.")}
          {info.platform === "windows" && line({ key: "potato" }, "Potato Mode", "Press to give the game everything the PC has. Press again to put it all back.")}
        </div>
      </Panel>

      <Panel title="Color keys" subtitle="Four keys that each jump straight to a preset.">
        <div className="bind-list">
          {[0, 1, 2, 3].map((i) => (
            <div className="bind-row" key={i}>
              <div className="row-text">
                <div className="row-title">Color key {i + 1}</div>
                <Select
                  label={`Preset for color key ${i + 1}`}
                  value={settings.colorKeys[i] ?? ""}
                  options={presetOptions}
                  onChange={(v) =>
                    update((s) => {
                      const colorKeys = [...s.colorKeys];
                      colorKeys[i] = v;
                      return { ...s, colorKeys };
                    })
                  }
                />
              </div>
              {bindButton({ key: "color", index: i })}
            </div>
          ))}
        </div>
      </Panel>

      <Panel title="Look wheel presets" subtitle={`Up to 8 presets around the wheel, in the order you pick them (${settings.wheelPresets.length}/8).`}>
        <div className="chip-picker">
          {wheelAll.map((p) => {
            const idx = settings.wheelPresets.indexOf(p.id);
            return (
              <button key={p.id} type="button" className={`pick ${idx >= 0 ? "active" : ""}`} onClick={() => toggleWheel(p.id)}>
                {idx >= 0 && <span className="pick-num">{idx + 1}</span>}
                {p.name}
                <span className="muted small">{"tone" in p ? "gamma" : "color"}</span>
              </button>
            );
          })}
        </div>
      </Panel>
    </div>
  );
}
