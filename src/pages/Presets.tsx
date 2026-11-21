import { useState } from "react";
import type { ColorPreset, GammaPreset } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { BUILTIN_COLOR, BUILTIN_GAMMA, newId } from "../lib/defaults";
import { NEUTRAL_COLOR, NEUTRAL_TONE } from "../lib/math";
import { applyColor, applyGamma, colorFromPayload, colorPayload, gammaPayload, toneFromPayload, toneOf } from "../lib/presets";
import { Panel, Segmented } from "../components/ui";
import { Thumb } from "../components/Preview";
import { Icon } from "../components/Icon";

type Kind = "gamma" | "color";

async function copy(text: string) {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}

export function PresetsPage() {
  const { settings, monitors, update, toast } = useStore();
  const [kind, setKind] = useState<Kind>("gamma");
  const [renaming, setRenaming] = useState<string | null>(null);
  const [name, setName] = useState("");
  const [code, setCode] = useState("");
  const [shared, setShared] = useState<{ id: string; code: string } | null>(null);
  const monId = settings.selectedMonitor || monitors[0]?.id || "";
  const mon = monitors.find((m) => m.id === monId);

  const list: (GammaPreset | ColorPreset)[] = kind === "gamma" ? settings.gammaPresets : settings.colorPresets;
  const isActive = (id: string) => (kind === "gamma" ? settings.activeGamma[monId] === id : settings.activeColor === id);

  const apply = (p: GammaPreset | ColorPreset) => {
    if (kind === "gamma") update((s) => applyGamma(s, p as GammaPreset, monId));
    else update((s) => applyColor(s, p as ColorPreset));
    toast(`${p.name} applied${kind === "gamma" ? ` on ${mon?.label ?? "the screen"}` : ""}.`, "good");
  };

  const saveCurrent = () => {
    const base = kind === "gamma" ? "My tone" : "My colour";
    const count = list.filter((p) => !p.builtin).length + 1;
    if (kind === "gamma") {
      const p: GammaPreset = { id: newId("ug"), name: `${base} ${count}`, builtin: false, tone: toneOf(settings, monId) };
      update((s) => ({ ...s, gammaPresets: [...s.gammaPresets, p], activeGamma: { ...s.activeGamma, [monId]: p.id } }));
    } else {
      const p: ColorPreset = { id: newId("uc"), name: `${base} ${count}`, builtin: false, color: settings.color };
      update((s) => ({ ...s, colorPresets: [...s.colorPresets, p], activeColor: p.id }));
    }
    toast("Saved as a new preset.", "good");
  };

  const remove = (id: string) =>
    update((s) =>
      kind === "gamma"
        ? { ...s, gammaPresets: s.gammaPresets.filter((p) => p.id !== id), wheelPresets: s.wheelPresets.filter((w) => w !== id) }
        : {
            ...s,
            colorPresets: s.colorPresets.filter((p) => p.id !== id),
            wheelPresets: s.wheelPresets.filter((w) => w !== id),
            colorKeys: s.colorKeys.map((k) => (k === id ? "" : k)),
          },
    );

  const rename = (id: string, value: string) => {
    const n = value.trim().slice(0, 40);
    setRenaming(null);
    if (!n) return;
    update((s) =>
      kind === "gamma"
        ? { ...s, gammaPresets: s.gammaPresets.map((p) => (p.id === id ? { ...p, name: n } : p)) }
        : { ...s, colorPresets: s.colorPresets.map((p) => (p.id === id ? { ...p, name: n } : p)) },
    );
  };

  const restore = () => {
    // Built-ins come back in their usual order (renamed ones keep their new name), yours stay after them.
    update((s) => ({
      ...s,
      gammaPresets: [...BUILTIN_GAMMA.map((b) => s.gammaPresets.find((p) => p.id === b.id) ?? b), ...s.gammaPresets.filter((p) => !p.builtin)],
      colorPresets: [...BUILTIN_COLOR.map((b) => s.colorPresets.find((p) => p.id === b.id) ?? b), ...s.colorPresets.filter((p) => !p.builtin)],
    }));
    toast("Built-in presets are all back.", "good");
  };

  const share = async (p: GammaPreset | ColorPreset) => {
    try {
      const c = await api.shareEncode("tone" in p ? gammaPayload(p.name, p.tone) : colorPayload(p.name, p.color));
      setShared({ id: p.id, code: c });
      toast((await copy(c)) ? "Share code copied." : "Share code ready below.", "good");
    } catch (e) {
      toast(errorText(e), "bad");
    }
  };

  const importCode = async () => {
    try {
      const p = await api.shareDecode(code);
      if (p.kind === "gamma") {
        const preset: GammaPreset = { id: newId("ug"), name: p.name || "Shared tone", builtin: false, tone: toneFromPayload(p) };
        update((s) => ({ ...s, gammaPresets: [...s.gammaPresets, preset] }));
        setKind("gamma");
      } else if (p.kind === "color") {
        const preset: ColorPreset = { id: newId("uc"), name: p.name || "Shared colour", builtin: false, color: colorFromPayload(p) };
        update((s) => ({ ...s, colorPresets: [...s.colorPresets, preset] }));
        setKind("color");
      } else {
        toast("That is a crosshair code. Paste it on the Crosshair page.", "info");
        return;
      }
      setCode("");
      toast(`Added "${p.name || "shared preset"}".`, "good");
    } catch (e) {
      toast(errorText(e), "bad");
    }
  };

  return (
    <div className="page">
      <Panel
        title="Presets"
        subtitle="Ready-made looks and your own. Tone presets go on the selected screen, colour presets on every screen."
        right={
          <Segmented
            value={kind}
            onChange={setKind}
            options={[
              { value: "gamma", label: `Gamma (${settings.gammaPresets.length})` },
              { value: "color", label: `Color (${settings.colorPresets.length})` },
            ]}
          />
        }
      >
        <div className="toolbar">
          <button type="button" className="btn primary" onClick={saveCurrent}>
            <Icon name="plus" size={15} /> Save current {kind === "gamma" ? "tone" : "colour"}
          </button>
          <button type="button" className="btn ghost" onClick={restore}>
            <Icon name="reset" size={15} /> Restore defaults
          </button>
          <div className="code-import">
            <input
              className="input"
              placeholder="Paste a LUNA- share code"
              value={code}
              onChange={(e) => setCode(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && code.trim() && importCode()}
            />
            <button type="button" className="btn" disabled={!code.trim()} onClick={importCode}>
              <Icon name="download" size={15} /> Add
            </button>
          </div>
        </div>

        <div className="card-grid">
          {list.map((p) => (
            <div key={p.id} className={`preset-card ${isActive(p.id) ? "active" : ""}`}>
              <button type="button" className="preset-thumb" onClick={() => apply(p)} title={`Apply ${p.name}`}>
                {"tone" in p ? (
                  <Thumb look={{ tone: p.tone, color: NEUTRAL_COLOR }} picture="dark" />
                ) : (
                  <Thumb look={{ tone: NEUTRAL_TONE, color: p.color }} picture="bright" />
                )}
                {isActive(p.id) && <span className="badge right">Active</span>}
              </button>
              <div className="preset-body">
                {renaming === p.id ? (
                  <input
                    className="input small"
                    autoFocus
                    value={name}
                    maxLength={40}
                    onChange={(e) => setName(e.target.value)}
                    onBlur={() => rename(p.id, name)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") rename(p.id, name);
                      if (e.key === "Escape") setRenaming(null);
                    }}
                  />
                ) : (
                  <div className="preset-name" title={p.name}>
                    {p.name}
                  </div>
                )}
                <div className="muted small mono">
                  {"tone" in p
                    ? `γ ${p.tone.gamma.toFixed(2)} · ${p.tone.contrast}% · ${p.tone.brightness >= 0 ? "+" : ""}${p.tone.brightness}`
                    : `hue ${p.color.hue}° · sat ${p.color.saturation}% · vib ${p.color.vibrance}%`}
                </div>
                <div className="preset-actions">
                  <button
                    type="button"
                    className="icon-btn"
                    title="Rename"
                    aria-label={`Rename ${p.name}`}
                    onClick={() => {
                      setRenaming(p.id);
                      setName(p.name);
                    }}
                  >
                    <Icon name="edit" size={15} />
                  </button>
                  <button type="button" className="icon-btn" title="Share code" aria-label={`Share ${p.name}`} onClick={() => share(p)}>
                    <Icon name="share" size={15} />
                  </button>
                  <button type="button" className="icon-btn danger" title="Delete" aria-label={`Delete ${p.name}`} onClick={() => remove(p.id)}>
                    <Icon name="trash" size={15} />
                  </button>
                  {!p.builtin && <span className="chip small">Yours</span>}
                </div>
                {shared?.id === p.id && (
                  <input className="input small mono" readOnly value={shared.code} onFocus={(e) => e.target.select()} aria-label="Share code" />
                )}
              </div>
            </div>
          ))}
          {list.length === 0 && <p className="muted">No presets here. Use Restore defaults to bring the built-in ones back.</p>}
        </div>
      </Panel>
    </div>
  );
}
