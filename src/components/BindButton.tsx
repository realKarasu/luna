import { useCallback, useEffect, useRef, useState } from "react";
import type { Bind, Binds } from "../types";
import { useStore } from "../store";
import { api } from "../lib/backend";
import { Icon } from "./Icon";

export type Slot = { key: "zoom" | "nvg" | "tuning" | "wheel" } | { key: "color"; index: number };

export const slotId = (s: Slot) => (s.key === "color" ? `color-${s.index}` : s.key);

export function getBind(b: Binds, s: Slot): Bind | null {
  return s.key === "color" ? b.color[s.index] ?? null : b[s.key];
}

export function setBind(b: Binds, s: Slot, v: Bind | null): Binds {
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

export type Recorder = {
  recording: Slot | null;
  start: (slot: Slot) => Promise<void>;
  cancel: () => void;
  assign: (slot: Slot, b: Bind) => void;
};

/** One recorder per page, so only one key box listens at a time. */
export function useBindRecorder(): Recorder {
  const { update, toast } = useStore();
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
    if (recRef.current) await api.cancelBindRecord();
    setRecording(slot);
    await api.startBindRecord();
  };
  const cancel = () => {
    api.cancelBindRecord();
    setRecording(null);
  };

  return { recording, start, cancel, assign };
}

export function BindButton({ slot, rec }: { slot: Slot; rec: Recorder }) {
  const { settings, update } = useStore();
  const b = getBind(settings.binds, slot);
  const on = rec.recording !== null && slotId(rec.recording) === slotId(slot);
  return (
    <div className="bind-control">
      <button type="button" className={`keycap ${on ? "recording" : ""} ${b ? "" : "empty"}`} onClick={() => rec.start(slot)}>
        {on ? "Press a key or mouse button..." : b?.label ?? "Not set"}
      </button>
      {b && !on && (
        <button type="button" className="icon-btn" aria-label="Clear key" title="Clear" onClick={() => update((s) => ({ ...s, binds: setBind(s.binds, slot, null) }))}>
          <Icon name="close" size={14} />
        </button>
      )}
      {on && (
        <>
          <button type="button" className="btn small" title="Use the left mouse button" onClick={() => rec.assign(slot, { vk: 1, label: "Mouse 1" })}>
            Mouse 1
          </button>
          <button type="button" className="btn small ghost" onClick={rec.cancel}>
            Cancel
          </button>
        </>
      )}
    </div>
  );
}
