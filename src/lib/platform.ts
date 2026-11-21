/** The system's everyday name, from the backend's "windows", "macos" or "linux". */
export function osName(platform: string): string {
  if (platform === "macos") return "macOS";
  if (platform === "linux") return "Linux";
  return "Windows";
}
