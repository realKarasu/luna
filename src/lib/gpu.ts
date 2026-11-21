import type { GpuVendor, MonitorInfo } from "../types";

const VENDOR_LABEL: Record<GpuVendor, string> = {
  nvidia: "NVIDIA",
  amd: "AMD",
  intel: "Intel",
  generic: "Other",
};

export function vendorLabel(v: GpuVendor | undefined): string {
  return VENDOR_LABEL[v ?? "generic"] ?? "Other";
}

/** "AMD Radeon RX 7800 XT", or a plain fallback when Windows named no card. */
export function gpuName(m: MonitorInfo): string {
  return m.gpu?.trim() || "Unknown graphics card";
}

export function usesDriver(m: MonitorInfo): boolean {
  return !!m.vibrance && m.vibrance !== "Color matrix";
}

/** One line on how vibrance works on this screen. */
export function vibranceNote(m: MonitorInfo | undefined): string {
  if (!m) return "Vibrance works as extra saturation.";
  if (m.vibrance === "NVIDIA digital vibrance")
    return "NVIDIA card: vibrance uses the driver's digital vibrance up to 200%, and a colour boost above that.";
  if (m.vibrance === "AMD saturation") return "AMD card: vibrance uses the driver's saturation setting, and a colour boost past its limit.";
  if (m.vendor === "nvidia" || m.vendor === "amd")
    return `${vendorLabel(m.vendor)} card, but its driver setting can't be used on every screen, so vibrance works as extra saturation.`;
  return "Vibrance works as extra saturation on this card.";
}

/** Short status text: which vibrance methods are in use. */
export function vibranceSummary(monitors: MonitorInfo[]): string {
  const kinds = new Set<string>();
  for (const m of monitors) {
    if (m.vibrance === "NVIDIA digital vibrance") kinds.add("NVIDIA vibrance");
    else if (m.vibrance === "AMD saturation") kinds.add("AMD saturation");
  }
  return kinds.size ? [...kinds].join(" + ") : "Color matrix";
}
