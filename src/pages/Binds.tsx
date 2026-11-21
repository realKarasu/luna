import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import type { Bind, Binds } from "../types";
import { useStore } from "../store";
import { api } from "../lib/backend";
import { Chip, Panel, Select } from "../components/ui";
import { Icon } from "../components/Icon";

type Slot = { key: "zoom" | "nvg" | "tuning" | "wheel" } | { key: "color"; index: number };

const slotId = (s: Slot) => (s.key === "color" ? `color-${s.index}` : s.key);

function getBind(b: Binds, s: Slot): Bind | null {
  return s.key === "color" ? b.color[s.index] ?? null : b[s.key];
}

function setBind(b: Binds, s: Slot, v: Bind | null): Binds {
  if (s.key === "color") {
    const color = [...b.color];
    color[s.index] = v;
    return { ...b, color };
  }
  return { ...b, [s.key]: v };
}

function allSlots(): Slot[] {
  return [{ key: "zoom" }, { key: "nvg" }, { key: "tuning" }, { key: "wheel" }, ...[0, 1, 2, 3].map((index) => ({ key: "color" as const, index }))];
}

export function BindsPage() {
  const { settings, update, toast } = useStore();
  const [recording, setRecording] = useState<Slot | null>(null);
  const recRef = useRef<Slot | null>(null);
  recRef.current = recording;

  const assign = useCallback(
    (slot: Slot, b: Bind) => {
      setRecording(null);
      api.cancelBindRecord();
      update((s) => {
        let binds = s.binds;
        // A key does one thing: take it off any other action first.
        for (const other of allSlots()) {
          if (slotId(other) !== slotId(slot) && getBind(binds, other)?.vk === b.vk) binds = setBind(binds, other, null);
        }
        return { ...s, binds: setBind(binds, slot, b) };
      });
      toast(`${b.label} is set.`, "good");
    },
    [update, toast],
  );

  useEffect(() => {
    const off = api.onBindRecorded((b) => {
      const slot = recRef.current;
      setRecording(null);
      if (slot && b) assign(slot, b);
    });
    return () => {
      off();
      api.cancelBindRecord();
    };
  }, [assign]);

  const start = async (slot: Slot) => {
    if (recording) await api.cancelBindRecord();
    setRecording(slot);
    await api.startBindRecord();
  };

  const presetOptions = [
    { value: "", label: "Nothing" },
    ...settings.colorPresets.map((p) => ({ value: p.id, label: `Color: ${p.name}` })),
    ...settings.gammaPresets.map((p) => ({ value: p.id, label: `Gamma: ${p.name}` })),
  ];

  const bindButton = (slot: Slot) => {
    const b = getBind(settings.binds, slot);
    const rec = recording && slotId(recording) === slotId(slot);
    return (
      <div className="bind-control">
        <button type="button" className={`keycap ${rec ? "recording" : ""} ${b ? "" : "empty"}`} onClick={() => start(slot)}>
          {rec ? "Press a key or mouse button..." : b?.label ?? "Not set"}
        </button>
        {b && !rec && (
          <button type="button" className="icon-btn" aria-label="Clear key" title="Clear" onClick={() => update((s) => ({ ...s, binds: setBind(s.binds, slot, null) }))}>
            <Icon name="close" size={14} />
          </button>
        )}
        {rec && (
          <>
            <button type="button" className="btn small" title="Use the left mouse button" onClick={() => assign(slot, { vk: 1, label: "Mouse 1" })}>
              Mouse 1
            </button>
            <button
              type="button"
              className="btn small ghost"
              onClick={() => {
                api.cancelBindRecord();
                setRecording(null);
              }}
            >
              Cancel
            </button>
          </>
        )}
      </div>
    );
  };

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
