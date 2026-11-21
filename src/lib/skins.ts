import type { Skin } from "../types";

export interface SkinInfo {
  id: Skin;
  name: string;
  blurb: string;
  /** Accent picked when the skin is chosen. */
  accent: string;
  /** Accents that suit the skin, shown first in the swatches. */
  accents: string[];
  /** Colors for the little sample card: background, surface, text, muted. */
  sample: [string, string, string, string];
  light?: boolean;
}

export const SKINS: SkinInfo[] = [
  {
    id: "nebula",
    name: "Nebula",
    blurb: "Night sky, glass panels, the original look",
    accent: "#a86dff",
    accents: [],
    sample: ["#0d0a16", "#1d1433", "#f3f1f8", "#8a8499"],
  },
  {
    id: "observatoire",
    name: "Observatoire",
    blurb: "Graphite night, quiet instruments, tick marks under the sliders",
    accent: "#b9c8e2",
    accents: ["#b9c8e2", "#e3c58f", "#9fd3c7"],
    sample: ["#0e0f11", "#15171a", "#e8e9eb", "#8a8f97"],
  },
  {
    id: "calibration",
    name: "Calibration",
    blurb: "Light measuring lab, tabs on top, grey step wedge",
    accent: "#c2410c",
    accents: ["#c2410c", "#1d4ed8", "#15803d"],
    sample: ["#f3f3f1", "#ffffff", "#16181c", "#6b7079"],
    light: true,
  },
  {
    id: "terrain",
    name: "Terrain",
    blurb: "Phosphor green, icon rail, sight brackets and keycaps",
    accent: "#9be37a",
    accents: ["#9be37a", "#e8c547", "#7ac7e3"],
    sample: ["#0f110f", "#151815", "#e4e9e1", "#8c948a"],
  },
  {
    id: "studio",
    name: "Studio",
    blurb: "Dense grading suite, histogram under the preview",
    accent: "#e5a54b",
    accents: ["#e5a54b", "#6fb3e8", "#e8e8e8"],
    sample: ["#1a1a1a", "#202020", "#ededed", "#6a6a6a"],
  },
];

export function skinInfo(id: string | undefined): SkinInfo {
  return SKINS.find((s) => s.id === id) ?? SKINS[0];
}
