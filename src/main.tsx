import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "@fontsource/hanken-grotesk/400.css";
import "@fontsource/hanken-grotesk/500.css";
import "@fontsource/hanken-grotesk/600.css";
import "@fontsource/hanken-grotesk/700.css";
import "@fontsource/unbounded/500.css";
import "@fontsource/unbounded/600.css";
import "@fontsource/jetbrains-mono/400.css";
// Skin fonts, latin only to keep the bundle small.
import "@fontsource/instrument-sans/latin-400.css";
import "@fontsource/instrument-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-400.css";
import "@fontsource/ibm-plex-sans/latin-500.css";
import "@fontsource/ibm-plex-sans/latin-600.css";
import "@fontsource/ibm-plex-mono/latin-400.css";
import "@fontsource/ibm-plex-mono/latin-500.css";
import "@fontsource/barlow-semi-condensed/latin-400.css";
import "@fontsource/barlow-semi-condensed/latin-500.css";
import "@fontsource/barlow-semi-condensed/latin-600.css";
import "@fontsource/geist-sans/latin-400.css";
import "@fontsource/geist-sans/latin-500.css";
import "@fontsource/geist-mono/latin-400.css";
import "@fontsource/geist-mono/latin-500.css";
import "./styles.css";
import "./skins.css";
import App from "./App";

// No right-click browser menu inside the app window.
if ("__TAURI_INTERNALS__" in window) {
  window.addEventListener("contextmenu", (e) => {
    const t = e.target as HTMLElement;
    if (!t.closest("input, textarea")) e.preventDefault();
  });
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
