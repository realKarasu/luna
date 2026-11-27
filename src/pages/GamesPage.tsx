import { useCallback, useEffect, useState } from "react";
import type { DisplayMode, FpsCorner, GameProfile, GamesStatus, InstalledGame, Settings, WindowInfo } from "../types";
import { useStore } from "../store";
import { api, errorText } from "../lib/backend";
import { DEFAULT_PROFILE } from "../lib/defaults";
import { GAMES_HELP } from "../lib/help";
import { HelpList } from "../components/Help";
import { BindButton, useBindRecorder } from "../components/BindButton";
import { Icon } from "../components/Icon";
import { Chip, Panel, Row, Select, Toggle } from "../components/ui";

const TWEAK_NAMES: Record<string, string> = {
  power: "high performance power plan",
  gameMode: "Game Mode",
  gameDvr: "no background recording",
  transparency: "no transparency",
  animations: "no animations",
};

const CORNERS: { value: FpsCorner; label: string }[] = [
  { value: "topLeft", label: "Top left" },
  { value: "topRight", label: "Top right" },
  { value: "bottomLeft", label: "Bottom left" },
  { value: "bottomRight", label: "Bottom right" },
];

const modeKey = (m: DisplayMode | null) => (m ? `${m.width}x${m.height}@${m.hz}` : "");
const sameExe = (a: string, b: string) => a !== "" && a.toLowerCase() === b.toLowerCase();
const modeLabel = (m: DisplayMode) => `${m.width} x ${m.height}, ${m.hz} Hz`;

function parseMode(key: string): DisplayMode | null {
  const m = /^(\d+)x(\d+)@(\d+)$/.exec(key);
  return m ? { width: Number(m[1]), height: Number(m[2]), hz: Number(m[3]) } : null;
}

/** Refresh rates the screen has at a lower resolution but not at its full one right now:
    the graphics card gives them back once another screen runs slower. */
function heldBack(list: DisplayMode[]): DisplayMode[] {
  const full = list.reduce<DisplayMode | null>((a, m) => (!a || m.width * m.height > a.width * a.height ? m : a), null);
  if (!full) return [];
  const have = new Set(list.filter((m) => m.width === full.width && m.height === full.height).map((m) => m.hz));
  const top = Math.max(...have);
  const rates = [...new Set(list.map((m) => m.hz))].filter((hz) => hz > top).sort((a, b) => b - a);
  return rates.map((hz) => ({ width: full.width, height: full.height, hz }));
}

function cleanExe(text: string): string {
  const name = text.trim().split(/[\\/]/).pop() ?? "";
  if (!name) return "";
  return /\.exe$/i.test(name) ? name : `${name}.exe`;
}

export function GamesPage() {
  const { settings, monitors, update, toast } = useStore();
  const rec = useBindRecorder();
  const [status, setStatus] = useState<GamesStatus | null>(null);
  const [fpsState, setFpsState] = useState<string>("off");
  const [windows, setWindows] = useState<WindowInfo[]>([]);
  const [installed, setInstalled] = useState<InstalledGame[] | null>(null);
  const [modes, setModes] = useState<Record<string, DisplayMode[]>>({});
  const [typed, setTyped] = useState("");
  const [busy, setBusy] = useState<string | null>(null);

  useEffect(() => {
    api.gamesStatus().then((s) => {
      setStatus(s);
      if (s.fps) setFpsState(s.fps.state);
    }, () => undefined);
    const offs = [
      api.onGameChanged(setStatus),
      api.onGameError((e) => toast(e, "bad")),
      api.onFpsState(setFpsState),
    ];
    return () => offs.forEach((off) => off());
  }, [toast]);

  const refreshWindows = useCallback(() => {
    api.borderlessWindows().then(setWindows, () => setWindows([]));
    api.installedGames().then(setInstalled, () => setInstalled([]));
  }, []);
  useEffect(refreshWindows, [refreshWindows]);

  // The screen a profile uses: its own, or the one selected on the Display page.
  const screenOf = useCallback(
    (p: GameProfile) => {
      const id = p.monitor || settings.selectedMonitor;
      return monitors.find((m) => m.id === id) ?? monitors.find((m) => m.primary) ?? monitors[0];
    },
    [monitors, settings.selectedMonitor],
  );

  // A profile can set every screen, so the modes of all of them are loaded.
  const wantedKey = settings.games.length ? monitors.map((m) => m.id).sort().join("|") : "";
  useEffect(() => {
    for (const id of wantedKey.split("|").filter(Boolean)) {
      if (modes[id]) continue;
      api.displayModes(id).then(
        (list) => setModes((m) => ({ ...m, [id]: list })),
        () => setModes((m) => ({ ...m, [id]: [] })),
      );
    }
  }, [wantedKey, modes]);

  const setGames = (fn: (g: GameProfile[]) => GameProfile[]) => update((s) => ({ ...s, games: fn(s.games) }));
  const edit = (exe: string, patch: Partial<GameProfile>) =>
    setGames((all) => all.map((g) => (sameExe(g.exe, exe) ? { ...g, ...patch } : g)));

  const add = (exe: string, name: string) => {
    if (!exe) return;
    if (settings.games.some((g) => sameExe(g.exe, exe))) {
      toast(`${name || exe} already has a profile.`, "info");
      return;
    }
    setGames((all) => [...all, { ...DEFAULT_PROFILE, exe, name }]);
    setTyped("");
  };

  const isBorderless = (exe: string) => settings.borderless.games.some((g) => sameExe(g.exe, exe));
  const setBorderless = (p: GameProfile, on: boolean) =>
    update((s: Settings) => {
      const rest = s.borderless.games.filter((g) => !sameExe(g.exe, p.exe));
      const games = on ? [...rest, { exe: p.exe, name: p.name || p.exe, fit: { ...s.borderless.fit } }] : rest;
      return { ...s, borderless: { ...s.borderless, games } };
    });

  const setPotato = async (on: boolean) => {
    setBusy("potato");
    try {
      setStatus(await api.potatoSet(on));
    } catch (e) {
      toast(errorText(e), "bad");
    } finally {
      setBusy(null);
    }
  };

  const restartAdmin = async () => {
    setBusy("admin");
    try {
      await api.restartAdmin("games");
    } catch (e) {
      toast(errorText(e), "bad");
      setBusy(null);
    }
  };

  const fps = settings.fps;
  const editFps = (patch: Partial<typeof fps>) => update((s) => ({ ...s, fps: { ...s.fps, ...patch } }));
  const potato = status?.potato;
  const active = status?.active ?? null;

  const gammaOptions = [{ value: "", label: "Leave it" }, ...settings.gammaPresets.map((g) => ({ value: g.id, label: g.name }))];
  const colorOptions = [{ value: "", label: "Leave it" }, ...settings.colorPresets.map((c) => ({ value: c.id, label: c.name }))];
  const crosshairOptions = [
    { value: "", label: "Leave it" },
    { value: "off", label: "Hidden" },
    { value: "on", label: "Shown, current design" },
    ...settings.crosshair.slots.flatMap((slot, i) =>
      slot ? [{ value: `slot${i}`, label: `Shown, ${settings.crosshair.slotNames[i] || `slot ${i + 1}`}` }] : [],
    ),
  ];
  const screenOptions = [
    { value: "", label: "The selected screen" },
    ...monitors.map((m) => ({ value: m.id, label: `${m.label} (${m.width} x ${m.height})` })),
  ];
  const fpsScreens = [
    { value: "", label: "The main screen" },
    ...monitors.map((m) => ({ value: m.id, label: `${m.label} (${m.width} x ${m.height})` })),
  ];

  const library = (installed ?? []).filter((g) => !settings.games.some((p) => sameExe(p.exe, g.exe)));
  const candidates = windows.filter((w) => w.exe && !settings.games.some((g) => sameExe(g.exe, w.exe)));
  const seen = new Set<string>();
  const pick = candidates.filter((w) => !seen.has(w.exe.toLowerCase()) && seen.add(w.exe.toLowerCase()));

  const potatoLine = () => {
    if (!potato?.on) return "Off. Nothing on the PC is changed.";
    const parts: string[] = [];
    if (potato.boosted) parts.push(`${potato.boosted} gets high priority`);
    if (potato.slowed) parts.push(`${potato.slowed} background programs slowed`);
    if (potato.tweaks.length) parts.push(potato.tweaks.map((t) => TWEAK_NAMES[t] ?? t).join(", "));
    const who = potato.manual ? "On" : `On for ${potato.game ?? "the game"}`;
    return parts.length ? `${who}: ${parts.join(", ")}.` : `${who}.`;
  };

  return (
    <div className="page games-page">
      <div className="xh-grid">
        <div className="zoom-side">
          <Panel
            title="Potato Mode"
            subtitle="Gives the game everything the PC has, and puts it all back when you turn it off."
            right={<Chip tone={potato?.on ? "good" : "plain"}>{potato?.on ? "On" : "Off"}</Chip>}
          >
            <Row title="Potato Mode" desc={potatoLine()}>
              <Toggle on={!!potato?.manual} disabled={busy === "potato"} label="Potato Mode" onChange={setPotato} />
            </Row>
            <Row title="Key" desc="Press it in game to switch Potato Mode.">
              <BindButton slot={{ key: "potato" }} rec={rec} />
            </Row>
            <p className="muted small games-note">
              The game in front gets high priority, other programs slow down, and five Optimizer switches go on if they were off. Voice chat, streaming and music apps are left alone.
            </p>
          </Panel>

          <Panel
            title="FPS counter"
            subtitle="Frames per second, frame time and 1% low of the game in front, in a corner of the screen. Add a graph or what the PC is doing."
            right={<Chip tone={fps.enabled ? (fpsState === "running" ? "good" : "warn") : "plain"}>{fps.enabled ? (fpsState === "running" ? "On" : fpsState === "needsAdmin" ? "Needs administrator" : fpsState === "error" ? "Unavailable" : "Starting") : "Off"}</Chip>}
          >
            <Row title="Show the counter">
              <Toggle on={fps.enabled} label="Show the FPS counter" onChange={(v) => editFps({ enabled: v })} />
            </Row>
            {fps.enabled && fpsState === "needsAdmin" && (
              <div className="games-admin">
                <div className="row-text">
                  <div className="row-title">Luna needs administrator to count frames</div>
                  <div className="muted small">Windows only shares frame timings with programs run as administrator. Luna opens again on this page.</div>
                </div>
                <button type="button" className="btn small" disabled={busy === "admin"} onClick={restartAdmin}>
                  Restart as administrator
                </button>
              </div>
            )}
            {fps.enabled && fpsState === "error" && (
              <p className="muted small games-note">Windows did not start frame tracing. Another frame counter may be using it; close it and turn the counter off and on.</p>
            )}
            <Row title="Frame time graph" desc="A line of the latest frames. Bumps are stutters.">
              <Toggle on={fps.graph} label="Frame time graph" onChange={(v) => editFps({ graph: v })} />
            </Row>
            <Row title="PC details" desc="Processor and graphics card load, card temperature, video memory and memory in use.">
              <Toggle on={fps.details} label="PC details" onChange={(v) => editFps({ details: v })} />
            </Row>
            <Row title="Corner">
              <Select label="FPS counter corner" value={fps.corner} options={CORNERS} onChange={(v) => editFps({ corner: v })} />
            </Row>
            <Row title="Screen">
              <Select label="FPS counter screen" value={fps.monitor} options={fpsScreens} onChange={(v) => editFps({ monitor: v })} />
            </Row>
            <Row title="Key" desc="Press it in game to show or hide the counter.">
              <BindButton slot={{ key: "fps" }} rec={rec} />
            </Row>
          </Panel>

          <Panel title="Questions">
            <HelpList items={GAMES_HELP} />
          </Panel>
        </div>

        <div className="zoom-side">
          <Panel
            title="Game profiles"
            subtitle="When the game starts, Luna sets its look and screen mode. When it closes, everything goes back."
            right={active ? <Chip tone="good">Playing {active.name}</Chip> : <Chip>No game running</Chip>}
          >
            <div className="games-add">
              <Select
                label="Add an installed game"
                value=""
                options={[
                  { value: "", label: installed === null ? "Looking for installed games..." : library.length ? "Add an installed game..." : "No installed games to add" },
                  ...library.map((g) => ({ value: g.exe, label: `${g.name} (${g.store})` })),
                ]}
                onChange={(exe) => {
                  const g = library.find((x) => x.exe === exe);
                  if (g) add(g.exe, g.name);
                }}
              />
            </div>
            <div className="games-add">
              <Select
                label="Add an open program"
                value=""
                options={[
                  { value: "", label: pick.length ? "Add an open program..." : "No open programs to add" },
                  ...pick.map((w) => ({ value: w.exe, label: `${w.title} (${w.exe})` })),
                ]}
                onChange={(exe) => {
                  const w = pick.find((x) => x.exe === exe);
                  if (w) add(w.exe, w.title);
                }}
              />
              <button type="button" className="icon-btn" aria-label="Refresh the installed games and open programs" title="Refresh" onClick={refreshWindows}>
                <Icon name="reset" size={16} />
              </button>
            </div>
            <form
              className="games-add"
              onSubmit={(e) => {
                e.preventDefault();
                const exe = cleanExe(typed);
                add(exe, exe.replace(/\.exe$/i, ""));
              }}
            >
              <input
                className="input grow"
                placeholder="Or type the program name, like game.exe"
                value={typed}
                aria-label="Program name"
                onChange={(e) => setTyped(e.target.value)}
              />
              <button type="submit" className="btn small" disabled={!cleanExe(typed)}>
                Add
              </button>
            </form>
          </Panel>

          {settings.games.length === 0 ? (
            <Panel>
              <p className="muted small">No profiles yet. Pick an installed game above, add a game that is open, or type its program name.</p>
            </Panel>
          ) : (
            settings.games.map((p) => {
              const screen = screenOf(p);
              const list = (screen && modes[screen.id]) || [];
              const current = modeKey(p.mode);
              const others = monitors.filter((m) => m.id !== screen?.id);
              const otherModes = p.otherModes.filter((o) => others.some((m) => m.id === o.monitor));
              // With another screen slowed down, the modes the card holds back can be picked too.
              const held = otherModes.length ? heldBack(list) : [];
              const known = [...held, ...list];
              const modeOptions = [
                { value: "", label: "Leave it" },
                ...(p.mode && !known.some((m) => modeKey(m) === current) ? [{ value: current, label: modeLabel(p.mode) }] : []),
                ...held.map((m) => ({ value: modeKey(m), label: `${modeLabel(m)} (not offered now)` })),
                ...list.map((m) => ({ value: modeKey(m), label: modeLabel(m) })),
              ];
              const setOther = (monitor: string, mode: DisplayMode | null) =>
                edit(p.exe, { otherModes: [...p.otherModes.filter((o) => o.monitor !== monitor), ...(mode ? [{ monitor, mode }] : [])] });
              const playing = !!active && sameExe(active.exe, p.exe);
              const title = p.name || p.exe;
              return (
                <Panel
                  key={p.exe}
                  className={`game-card ${p.enabled ? "" : "off"}`}
                  title={
                    <span className="game-title">
                      {title}
                      {playing && <Chip tone="good">Running</Chip>}
                    </span>
                  }
                  subtitle={<span className="mono">{p.exe}</span>}
                  right={
                    <div className="win-actions">
                      <Toggle on={p.enabled} label={`Profile for ${title}`} onChange={(v) => edit(p.exe, { enabled: v })} />
                      <button
                        type="button"
                        className="icon-btn danger"
                        aria-label={`Delete the profile for ${title}`}
                        title="Delete"
                        onClick={() => setGames((all) => all.filter((g) => !sameExe(g.exe, p.exe)))}
                      >
                        <Icon name="trash" size={15} />
                      </button>
                    </div>
                  }
                >
                  <div className="game-grid">
                    <label className="field">
                      <span className="muted small">Screen</span>
                      <Select label={`Screen for ${title}`} value={p.monitor} options={screenOptions} onChange={(v) => edit(p.exe, { monitor: v, mode: null, otherModes: [] })} />
                    </label>
                    <label className="field">
                      <span className="muted small">Resolution and refresh</span>
                      <Select
                        label={`Resolution for ${title}`}
                        value={current}
                        options={modeOptions}
                        onChange={(v) => edit(p.exe, { mode: parseMode(v) })}
                      />
                    </label>
                    <label className="field">
                      <span className="muted small">Gamma preset</span>
                      <Select label={`Gamma preset for ${title}`} value={p.gamma} options={gammaOptions} onChange={(v) => edit(p.exe, { gamma: v })} />
                    </label>
                    <label className="field">
                      <span className="muted small">Color preset</span>
                      <Select label={`Color preset for ${title}`} value={p.color} options={colorOptions} onChange={(v) => edit(p.exe, { color: v })} />
                    </label>
                    <label className="field">
                      <span className="muted small">Crosshair</span>
                      <Select label={`Crosshair for ${title}`} value={p.crosshair} options={crosshairOptions} onChange={(v) => edit(p.exe, { crosshair: v })} />
                    </label>
                  </div>
                  {others.length > 0 && (
                    <div className="game-others">
                      <div className="muted small game-others-title">Other screens while the game runs</div>
                      <div className="game-grid">
                        {others.map((m) => {
                          const set = otherModes.find((o) => o.monitor === m.id)?.mode ?? null;
                          const theirs = modes[m.id] || [];
                          const key = modeKey(set);
                          return (
                            <label className="field" key={m.id}>
                              <span className="muted small">{m.label}</span>
                              <Select
                                label={`${m.label} while ${title} runs`}
                                value={key}
                                options={[
                                  { value: "", label: "Leave it" },
                                  ...(set && !theirs.some((x) => modeKey(x) === key) ? [{ value: key, label: modeLabel(set) }] : []),
                                  ...theirs.map((x) => ({ value: modeKey(x), label: modeLabel(x) })),
                                ]}
                                onChange={(v) => setOther(m.id, parseMode(v))}
                              />
                            </label>
                          );
                        })}
                      </div>
                    </div>
                  )}
                  <div className="game-switches">
                    <Row title="Potato Mode" desc="While the game runs.">
                      <Toggle on={p.potato} label={`Potato Mode for ${title}`} onChange={(v) => edit(p.exe, { potato: v })} />
                    </Row>
                    <Row title="Borderless" desc="Takes the frame off when it opens, with the size from the Borderless page.">
                      <Toggle on={isBorderless(p.exe)} label={`Borderless for ${title}`} onChange={(v) => setBorderless(p, v)} />
                    </Row>
                  </div>
                </Panel>
              );
            })
          )}
        </div>
      </div>
    </div>
  );
}
