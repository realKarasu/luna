import { useState } from "react";
import type { Crosshair } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { CROSSHAIR_COLORS, CROSSHAIR_DESIGNS, DEFAULT_CROSSHAIR } from "../lib/defaults";
import { crosshairFromPayload, crosshairPayload } from "../lib/presets";
import { refById } from "../lib/refs";
import { Panel, Row, Select, Slider, Toggle } from "../components/ui";
import { CrosshairSvg } from "../components/CrosshairSvg";
import { Icon } from "../components/Icon";

export function CrosshairPage() {
  const { settings, monitors, update, toast } = useStore();
  const ch = settings.crosshair;
  const c = ch.current;
  const [code, setCode] = useState("");
  const [shareCode, setShareCode] = useState("");
  const [slot, setSlot] = useState(0);
  const bg = refById("dark")?.url;

  const edit = (patch: Partial<Crosshair>) => update((s) => ({ ...s, crosshair: { ...s.crosshair, current: { ...s.crosshair.current, ...patch } } }));
  const setCh = (patch: Partial<typeof ch>) => update((s) => ({ ...s, crosshair: { ...s.crosshair, ...patch } }));

  const saveSlot = (i: number) => {
    const slots = [...ch.slots];
    slots[i] = { ...c };
    const names = [...ch.slotNames];
    if (!names[i]) names[i] = `Slot ${i + 1}`;
    setCh({ slots, slotNames: names });
    toast(`Saved to ${names[i]}.`, "good");
  };
  const loadSlot = (i: number) => {
    const s = ch.slots[i];
    if (s) setCh({ current: { ...s } });
  };
  const clearSlot = (i: number) => {
    const slots = [...ch.slots];
    slots[i] = null;
    const names = [...ch.slotNames];
    names[i] = "";
    setCh({ slots, slotNames: names });
  };

  const share = async () => {
    try {
      const name = ch.slotNames[slot] || "Crosshair";
      const out = await api.shareEncode(crosshairPayload(name, c));
      setShareCode(out);
      try {
        await navigator.clipboard.writeText(out);
        toast("Crosshair code copied.", "good");
      } catch {
        toast("Crosshair code ready below.", "good");
      }
    } catch (e) {
      toast(errorText(e), "bad");
    }
  };
  const importCode = async () => {
    try {
      const p = await api.shareDecode(code);
      if (p.kind !== "crosshair") {
        toast("That code is a display preset. Paste it on the Presets page.", "info");
        return;
      }
      setCh({ current: crosshairFromPayload(p) });
      setCode("");
      toast(`Loaded "${p.name || "crosshair"}".`, "good");
    } catch (e) {
      toast(errorText(e), "bad");
    }
  };

  const monitorOptions = [{ value: "", label: "Primary display" }, ...monitors.map((m) => ({ value: m.id, label: m.label }))];

  return (
    <div className="page crosshair-page">
      <div className="xh-grid">
        <Panel title="Crosshair" subtitle="An always-on-top sight in the middle of the screen. Mouse clicks go straight through it.">
          <div className="xh-stage" style={bg ? { backgroundImage: `url(${bg})` } : undefined}>
            <CrosshairSvg c={c} size={256} />
          </div>
          <Row title="Show crosshair" desc={ch.enabled ? "Showing now." : "Hidden."}>
            <Toggle on={ch.enabled} label="Show crosshair" onChange={(v) => setCh({ enabled: v })} />
          </Row>
          <Row title="Screen">
            <Select label="Crosshair screen" value={ch.monitor} options={monitorOptions} onChange={(v) => setCh({ monitor: v })} />
          </Row>
          <Row title="Global shortcut" desc="Ctrl + Alt + Shift + F11 shows or hides it from anywhere.">
            <Toggle on={ch.hotkey} label="Global shortcut" onChange={(v) => setCh({ hotkey: v })} />
          </Row>
        </Panel>

        <Panel title="Design" right={<button type="button" className="btn ghost small" onClick={() => setCh({ current: { ...DEFAULT_CROSSHAIR } })}><Icon name="reset" size={14} /> Default</button>}>
          <div className="design-grid">
            {CROSSHAIR_DESIGNS.map((d) => (
              <button key={d.id} type="button" className={`design-card ${c.design === d.id ? "active" : ""}`} onClick={() => edit({ design: d.id })} title={d.name}>
                <CrosshairSvg c={{ ...c, design: d.id, opacity: 100 }} size={64} />
                <span>{d.name}</span>
              </button>
            ))}
          </div>
          <div className="swatches">
            {CROSSHAIR_COLORS.map((col) => (
              <button
                key={col.hex}
                type="button"
                className={`swatch ${c.color.toLowerCase() === col.hex ? "active" : ""}`}
                style={{ background: col.hex }}
                title={col.name}
                aria-label={col.name}
                onClick={() => edit({ color: col.hex })}
              />
            ))}
            <label className="swatch custom" title="Any colour">
              <input type="color" value={c.color} onChange={(e) => edit({ color: e.target.value })} aria-label="Any colour" />
            </label>
          </div>
          <div className="xh-sliders">
            <Slider label="Ring size" value={c.ring} min={2} max={40} step={1} def={10} unit="px" onChange={(v) => edit({ ring: v })} />
            <Slider label="Arm length" value={c.length} min={0} max={40} step={1} def={8} unit="px" onChange={(v) => edit({ length: v })} />
            <Slider label="Gap" value={c.gap} min={0} max={30} step={1} def={4} unit="px" onChange={(v) => edit({ gap: v })} />
            <Slider label="Thickness" value={c.thickness} min={0.5} max={10} step={0.5} def={2} unit="px" onChange={(v) => edit({ thickness: v })} />
            <Slider label="Centre dot" value={c.dot} min={0} max={10} step={0.5} def={2} unit="px" onChange={(v) => edit({ dot: v })} />
            <Slider label="Outline" value={c.outline} min={0} max={4} step={0.5} def={1} unit="px" onChange={(v) => edit({ outline: v })} />
            <Slider label="Opacity" value={c.opacity} min={5} max={100} step={1} def={100} unit="%" onChange={(v) => edit({ opacity: v })} />
          </div>
        </Panel>
      </div>

      <Panel title="Saved crosshairs" subtitle="Eight slots. Click a filled slot to load it.">
        <div className="slot-grid">
          {ch.slots.map((s, i) => (
            <div key={i} className={`slot ${slot === i ? "active" : ""} ${s ? "filled" : ""}`} onClick={() => setSlot(i)}>
              <button type="button" className="slot-view" onClick={() => s && loadSlot(i)} aria-label={s ? `Load ${ch.slotNames[i]}` : `Empty slot ${i + 1}`}>
                {s ? <CrosshairSvg c={{ ...s, opacity: 100 }} size={56} /> : <span className="muted small">Empty</span>}
              </button>
              <input
                className="input small"
                value={ch.slotNames[i]}
                placeholder={`Slot ${i + 1}`}
                maxLength={24}
                aria-label={`Name of slot ${i + 1}`}
                onChange={(e) => {
                  const names = [...ch.slotNames];
                  names[i] = e.target.value;
                  setCh({ slotNames: names });
                }}
              />
              <div className="slot-actions">
                <button type="button" className="btn small" onClick={() => saveSlot(i)}>
                  Save here
                </button>
                {s && (
                  <button type="button" className="icon-btn danger" aria-label={`Clear slot ${i + 1}`} onClick={() => clearSlot(i)}>
                    <Icon name="trash" size={14} />
                  </button>
                )}
              </div>
            </div>
          ))}
        </div>
        <div className="toolbar">
          <button type="button" className="btn" onClick={share}>
            <Icon name="share" size={15} /> Share this crosshair
          </button>
          {shareCode && <input className="input mono grow" readOnly value={shareCode} onFocus={(e) => e.target.select()} aria-label="Crosshair share code" />}
          <div className="code-import">
            <input className="input" placeholder="Paste a LUNA-X code" value={code} onChange={(e) => setCode(e.target.value)} onKeyDown={(e) => e.key === "Enter" && code.trim() && importCode()} />
            <button type="button" className="btn" disabled={!code.trim()} onClick={importCode}>
              <Icon name="download" size={15} /> Load
            </button>
          </div>
        </div>
      </Panel>
    </div>
  );
}
