// Simple line icons drawn for Luna (24 x 24 grid, stroke based).
const PATHS: Record<string, string> = {
  display: "M3 5h18v11H3z M8 20h8 M12 16v4",
  presets: "M4 4h7v7H4z M13 4h7v7h-7z M4 13h7v7H4z M13 13h7v7h-7z",
  crosshair: "M12 3v5 M12 16v5 M3 12h5 M16 12h5 M12 12h.01 M12 7a5 5 0 1 0 .01 0",
  optimizer: "M13 2 4 14h7l-1 8 9-12h-7z",
  binds: "M3 7h18v10H3z M7 11h.01 M11 11h.01 M15 11h.01 M8 14h8",
  theme: "M12 3a9 9 0 1 0 9 9c0-1-1-1.5-2-1.5h-2.5a2 2 0 0 1-1.6-3.2l.4-.6c.7-1-.1-2.7-1.3-2.7z M7.5 11h.01 M10 7.5h.01 M15 7.5h.01",
  settings:
    "M12 9a3 3 0 1 0 .01 0 M19.4 15a1.6 1.6 0 0 0 .3 1.8l.1.1-2 3.4-.2-.1a1.6 1.6 0 0 0-1.9.1 7 7 0 0 1-1.6.9 1.6 1.6 0 0 0-1 1.5V23H9v-.3a1.6 1.6 0 0 0-1-1.5 7 7 0 0 1-1.6-.9 1.6 1.6 0 0 0-1.9-.1l-.2.1-2-3.4.1-.1a1.6 1.6 0 0 0 .3-1.8 7 7 0 0 1 0-2 1.6 1.6 0 0 0-.3-1.8l-.1-.1 2-3.4.2.1a1.6 1.6 0 0 0 1.9-.1A7 7 0 0 1 8 7.8a1.6 1.6 0 0 0 1-1.5V6h4v.3a1.6 1.6 0 0 0 1 1.5 7 7 0 0 1 1.6.9 1.6 1.6 0 0 0 1.9.1l.2-.1 2 3.4-.1.1a1.6 1.6 0 0 0-.3 1.8 7 7 0 0 1 0 2z",
  chevron: "M6 9l6 6 6-6",
  minimize: "M6 12h12",
  maximize: "M6 6h12v12H6z",
  close: "M6 6l12 12 M18 6 6 18",
  reset: "M4 12a8 8 0 1 0 2.3-5.6 M4 4v4h4",
  plus: "M12 5v14 M5 12h14",
  trash: "M4 7h16 M9 7V4h6v3 M6 7l1 13h10l1-13",
  edit: "M4 20h4L19 9l-4-4L4 16z",
  share: "M8 12h8 M12 8l4 4-4 4 M4 4v16",
  copy: "M8 8h11v11H8z M5 16V5h11",
  check: "M5 12l5 5 9-10",
  download: "M12 4v11 M7 10l5 5 5-5 M5 20h14",
  image: "M3 5h18v14H3z M3 16l5-5 4 4 3-3 6 6 M15.5 9h.01",
  folder: "M3 6h6l2 2h10v11H3z",
  sparkle: "M12 3l1.8 5.4L19 10l-5.2 1.6L12 17l-1.8-5.4L5 10l5.2-1.6z M19 16l.7 2 2 .7-2 .7-.7 2-.7-2-2-.7 2-.7z",
  broom: "M14 3l-4 9 M6 13h8l2 8H4z M8 17v4 M12 17v4",
  mouse: "M8 4h8v16H8z M12 4v6",
  borderless: "M4 9V4h5 M15 4h5v5 M20 15v5h-5 M9 20H4v-5 M9 9h6v6H9z",
  zoom: "M10 4a6 6 0 1 0 .01 0 M15 15l5 5 M10 7v6 M7 10h6",
  moon: "M20 14.5A8.5 8.5 0 1 1 9.5 4a7 7 0 0 0 10.5 10.5z",
  eye: "M2 12s3.5-7 10-7 10 7 10 7-3.5 7-10 7S2 12 2 12z M12 9a3 3 0 1 0 .01 0",
  power: "M12 3v8 M6.3 7a8 8 0 1 0 11.4 0",
  info: "M12 3a9 9 0 1 0 .01 0 M12 11v6 M12 7.5h.01",
  wheel: "M12 3a9 9 0 1 0 .01 0 M12 3v18 M3 12h18 M5.6 5.6l12.8 12.8 M18.4 5.6 5.6 18.4",
};

export function Icon({ name, size = 18, className }: { name: string; size?: number; className?: string }) {
  const d = PATHS[name] ?? PATHS.info;
  return (
    <svg
      className={className}
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={d} />
    </svg>
  );
}

export function MoonLogo({ size = 26 }: { size?: number }) {
  return (
    <svg width={size} height={size} viewBox="0 0 64 64" aria-hidden="true">
      <defs>
        <linearGradient id="luna-moon" x1="0" y1="0" x2="1" y2="1">
          <stop offset="0" stopColor="#f1e6ff" />
          <stop offset="0.55" stopColor="var(--accent-2, #d4b6ff)" />
          <stop offset="1" stopColor="var(--accent, #a86dff)" />
        </linearGradient>
      </defs>
      <path
        d="M44 50.5A22 22 0 1 1 30 9a17.5 17.5 0 0 0 20.7 29.2A22 22 0 0 1 44 50.5z"
        fill="url(#luna-moon)"
      />
      <circle cx="47" cy="14" r="2.2" fill="#f1e6ff" />
      <circle cx="54.5" cy="25" r="1.4" fill="#f1e6ff" opacity="0.8" />
    </svg>
  );
}
