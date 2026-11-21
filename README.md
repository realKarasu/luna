# Luna

Luna is a free, open-source Windows 10/11 (x64) app that tunes each of your screens for PC games, draws a crosshair overlay and applies a set of Windows gaming tweaks. Everything runs locally: no account, no licence key, no payment and no telemetry. The only network use is the update check on this repository's releases, which can be turned off. It runs without administrator rights.

> Luna is an independent project. It is not affiliated with, endorsed by or connected to any other display tuning product or company.

## Features

### Display tuning (per screen)
- **Tone**: gamma (0.60 to 3.00), brightness (-20 to +20), contrast (70 to 130 %) and red, green and blue gain (20 to 180 %), applied through the screen's gamma ramp. A live curve graph shows the result against a dashed identity line.
- **Color**: hue (-180 to +180) and saturation (0 to 350 %) through the Windows colour effect, plus vibrance (0 to 250 %). Luna finds the graphics card behind each screen by itself (NVIDIA, AMD, Intel or other) and refreshes it when Windows changes the screens. Vibrance uses the driver's own setting where there is one: NVIDIA digital vibrance or AMD saturation, both loaded at run time with nothing to install. Whatever the driver can't reach (above 200 % on most cards), and every screen on Intel or other cards, gets a matching saturation boost instead. The driver values found at start are put back on quit, on Reset and after a crash.
- **Before/after preview** on reference pictures with a draggable split. The preview uses exactly the same math as the real output.
- Originals are saved at start and put back on quit, on crashes (a restore file is replayed on the next start) and from the Reset button. A master **Tuning on/off** switch.

### Presets
- 15 built-in gamma presets and 12 colour presets as cards with live thumbnails.
- Save your own, rename, delete, and **Restore defaults** to bring deleted built-ins back.
- **Share codes** (`LUNA-G-...`, `LUNA-C-...`, `LUNA-X-...`) with a checksum, so a mistyped code is caught.

### Game tools
- **Ultra Zoom**: hold a key or mouse button to magnify the middle of a chosen screen (1.5x to 8x).
- **Rust profile** only:
  - **Green NVG** and **Blue NVG**: gamma 3.00 with a tint while a bind is held, on a chosen screen.
  - **Green True Black** and **Red Black Holo**: a colour matrix that pushes a sight's colour toward black.
- **Binds**: keyboard keys or mouse buttons 1 to 5 for Zoom, NVG, Tuning on/off, four colour keys and the look wheel. Click a key box and press the key to record it.
- **Look wheel**: hold F7 (changeable), point with the mouse at one of up to 8 presets, release to apply.

### Crosshair overlay
- Transparent, click-through, always on top, on the screen you choose.
- 10 designs, 8 colours (or any colour), sliders for ring, arm length, gap, thickness, centre dot, outline and opacity.
- 8 save slots, share codes, and an optional global **Ctrl + Alt + Shift + F11** toggle (off by default).

### Optimizer
- **Optimize all** and **Undo all**, plus 10 switches: High performance power plan, Game Mode, background recording off, optimizations for windowed games, mouse acceleration off, Sticky Keys popup off, notifications off, background apps off, transparency effects off and window animations off.
- Every switch reads the current state and backs it up before changing anything, so Undo restores exactly what you had. Only per-user (HKCU) settings are changed.
- **Clean temp files** deletes only files older than one day from your temp folder and reports the space freed.

### Theme and settings
- Five styles that change the whole look of the app, each with its own fonts and one detail of its own:
  - **Nebula** (default): animated night-sky background with its own moon, or your own picture or GIF, glass panels, plus glow amount, font and corner roundness.
  - **Observatoire**: graphite night, quiet instruments, tick marks under every slider.
  - **Calibration**: light measuring lab, pages as tabs on top, a grey step wedge and channel ramps under the preview.
  - **Terrain**: phosphor green, icon rail, sight brackets on the panels, segmented sliders and a readout over the preview.
  - **Studio**: dense grading suite with a live histogram under the preview.
- Accent colour for every style.
- Start with Windows, start minimized to the tray, close to tray, and a tray icon with quick toggles.

### Updates
- Luna checks this repository's latest release a few seconds after it starts, then every 6 hours.
- A new version is downloaded and its signature is checked against the public key built into the app, so only installers signed by the project are accepted.
- Found at start: it installs right away and Luna opens again. Found later: it waits and installs when Luna quits, or at once from **Restart to update**, so a game is never cut off.
- **Automatic updates** can be turned off in Settings; **Check now** still works.

Settings are saved as JSON in `%APPDATA%\Luna`.

## Building

Requirements: Windows 10 or 11 (x64), [Node.js](https://nodejs.org/) 18 or newer, the [Rust toolchain](https://rustup.rs/) (MSVC) and the WebView2 runtime (already part of Windows 11).

```sh
npm install
npm run tauri build
```

The installers end up in `src-tauri/target/release/bundle/` (NSIS and MSI, per-user install).

Other useful commands:

```sh
npm run build                     # type-check and build the interface only
cd src-tauri && cargo test        # unit tests: gamma ramps, colour matrices, vibrance ranges, GPU makers, share codes, settings
npm run tauri dev                 # run in development mode
```

### Publishing an update

The release script bumps the version, builds and signs the installers, writes `latest.json` and creates the GitHub release that every installed copy reads:

```sh
npm run release -- 0.4.0 "What changed, in one or two lines"
```

It needs the [GitHub CLI](https://cli.github.com/) signed in and the updater private key at `%USERPROFILE%\.tauri\luna-updater.key` (or in `TAURI_SIGNING_PRIVATE_KEY`). That key never goes in the repository; without it no update can be signed.

Opening `dist/index.html` from a plain web server shows the interface in a browser preview mode with a demo backend (nothing on your PC is changed).

## How it works

| Part | Windows API |
| --- | --- |
| Tone | `SetDeviceGammaRamp` per display device |
| Hue, saturation, holo | `MagSetFullscreenColorEffect` (Magnification API) |
| GPU per screen | DXGI adapters and outputs (`CreateDXGIFactory1`), matched by device name; refreshed on `WM_DISPLAYCHANGE` |
| Vibrance, NVIDIA | NVAPI digital vibrance, loaded dynamically from `nvapi64.dll` |
| Vibrance, AMD | AMD Display Library saturation (`ADL2_Display_Color_Set`), loaded dynamically from `atiadlxx.dll` or `atiadlxy.dll` |
| Vibrance, Intel and others | Saturation matrix through `MagSetFullscreenColorEffect` |
| Ultra Zoom | `MagSetFullscreenTransform` |
| Binds and look wheel | Low-level keyboard and mouse hooks on their own thread |
| Optimizer | Per-user registry values, `powercfg`, `SystemParametersInfo` |
| Updates | `tauri-plugin-updater`: HTTPS GET of `latest.json`, minisign signature check, NSIS installer in passive mode |

Windows limits how far a gamma ramp may move from neutral for programs without admin rights. When a curve is refused, Luna applies the strongest version Windows accepts and tells you how much of it is active.

## Licence

MIT. See [LICENSE](LICENSE).
