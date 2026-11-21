import { createContext, useCallback, useContext, useEffect, useMemo, useRef, useState, type ReactNode } from "react";
import type { AppState, MonitorInfo, Settings } from "./types";
import { api, errorText, IDLE_UPDATE } from "./lib/backend";
import { normalize } from "./lib/defaults";

interface Toast {
  id: number;
  text: string;
  tone: "good" | "bad" | "info";
}

interface Store {
  ready: boolean;
  settings: Settings;
  monitors: MonitorInfo[];
  info: Omit<AppState, "settings" | "monitors">;
  /** Changes settings; the backend gets them a moment later (batched). */
  update: (fn: (s: Settings) => Settings) => void;
  /** Replaces settings from the backend without sending them back. */
  replace: (s: Settings) => void;
  toasts: Toast[];
  toast: (text: string, tone?: Toast["tone"]) => void;
  setInfo: (patch: Partial<Store["info"]>) => void;
}

const Ctx = createContext<Store | null>(null);

export function useStore(): Store {
  const s = useContext(Ctx);
  if (!s) throw new Error("Store missing");
  return s;
}

const SEND_EVERY_MS = 30;

export function StoreProvider({ children }: { children: ReactNode }) {
  const [ready, setReady] = useState(false);
  const [settings, setSettings] = useState<Settings>(() => normalize({} as Settings));
  const [monitors, setMonitors] = useState<MonitorInfo[]>([]);
  const [info, setInfoState] = useState<Store["info"]>({
    driverVibrance: false,
    magnifier: true,
    gammaStrength: {},
    version: "0.3.0",
    dataDir: "",
    autostart: false,
    update: IDLE_UPDATE,
  });
  const [toasts, setToasts] = useState<Toast[]>([]);
  const latest = useRef(settings);
  const pending = useRef<number | null>(null);
  const inFlight = useRef(false);
  const again = useRef(false);

  const toast = useCallback((text: string, tone: Toast["tone"] = "info") => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t.slice(-3), { id, text, tone }]);
    window.setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 3600);
  }, []);

  const send = useCallback(async () => {
    pending.current = null;
    if (inFlight.current) {
      again.current = true;
      return;
    }
    inFlight.current = true;
    try {
      const strength = await api.setSettings(latest.current);
      setInfoState((i) => ({ ...i, gammaStrength: strength ?? {} }));
    } catch (e) {
      toast(errorText(e), "bad");
    } finally {
      inFlight.current = false;
      if (again.current) {
        again.current = false;
        send();
      }
    }
  }, [toast]);

  const update = useCallback(
    (fn: (s: Settings) => Settings) => {
      const next = fn(latest.current);
      if (next === latest.current) return;
      latest.current = next;
      setSettings(next);
      if (pending.current === null) pending.current = window.setTimeout(send, SEND_EVERY_MS);
    },
    [send],
  );

  const replace = useCallback((s: Settings) => {
    const n = normalize(s);
    latest.current = n;
    setSettings(n);
  }, []);

  const setInfo = useCallback((patch: Partial<Store["info"]>) => setInfoState((i) => ({ ...i, ...patch })), []);

  useEffect(() => {
    let alive = true;
    api
      .getState()
      .then((st) => {
        if (!alive) return;
        const n = normalize(st.settings);
        if (!n.selectedMonitor && st.monitors[0]) n.selectedMonitor = st.monitors[0].id;
        latest.current = n;
        setSettings(n);
        setMonitors(st.monitors);
        const { settings: _s, monitors: _m, ...rest } = st;
        setInfoState(rest);
        setReady(true);
        // Seeding or filling defaults changed something: store it.
        if (JSON.stringify(n) !== JSON.stringify(st.settings)) send();
      })
      .catch((e) => {
        toast(errorText(e), "bad");
        setReady(true);
      });
    const offs = [
      api.onSettingsChanged((s) => replace(s)),
      api.onMonitorsChanged((m) => setMonitors(m)),
      api.onUpdateStatus((u) => setInfoState((i) => ({ ...i, update: u }))),
    ];
    return () => {
      alive = false;
      offs.forEach((o) => o());
    };
  }, [replace, send, toast]);

  const value = useMemo(
    () => ({ ready, settings, monitors, info, update, replace, toasts, toast, setInfo }),
    [ready, settings, monitors, info, update, replace, toasts, toast, setInfo],
  );
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>;
}
