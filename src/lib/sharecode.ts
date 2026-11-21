// Port of src-tauri/src/sharecode.rs, used only by the browser demo.
// Inside the app the Rust implementation (unit tested) does the work.
import type { SharePayload } from "../types";

const LETTERS: Record<string, string> = { gamma: "G", color: "C", crosshair: "X" };
const KINDS: Record<string, SharePayload["kind"]> = { G: "gamma", C: "color", X: "crosshair" };

function crc16(data: number[]): number {
  let crc = 0xffff;
  for (const b of data) {
    crc ^= b << 8;
    for (let i = 0; i < 8; i++) crc = crc & 0x8000 ? ((crc << 1) ^ 0x1021) & 0xffff : (crc << 1) & 0xffff;
  }
  return crc;
}

function b64urlEncode(bytes: number[]): string {
  let s = "";
  for (const b of bytes) s += String.fromCharCode(b);
  return btoa(s).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function b64urlDecode(text: string): number[] {
  if (!/^[A-Za-z0-9_-]*$/.test(text)) throw new Error("The code has characters that don't belong in it.");
  const b = text.replace(/-/g, "+").replace(/_/g, "/");
  const bin = atob(b + "===".slice((b.length + 3) % 4));
  return Array.from(bin, (c) => c.charCodeAt(0));
}

function pushVarint(out: number[], v: number) {
  let z = v >= 0 ? v * 2 : -v * 2 - 1;
  for (;;) {
    const byte = z % 128;
    z = Math.floor(z / 128);
    if (z === 0) {
      out.push(byte);
      return;
    }
    out.push(byte | 0x80);
  }
}

function readVarint(data: number[], pos: { i: number }): number {
  let z = 0;
  let mult = 1;
  for (;;) {
    if (pos.i >= data.length) throw new Error("The code is cut short.");
    const b = data[pos.i++];
    z += (b & 0x7f) * mult;
    if (!(b & 0x80)) break;
    mult *= 128;
    if (mult > 2 ** 56) throw new Error("The code has a broken number.");
  }
  return z % 2 === 0 ? z / 2 : -(z + 1) / 2;
}

export function encodeShare(p: SharePayload): string {
  const letter = LETTERS[p.kind];
  if (!letter) throw new Error("Unknown share kind.");
  const bytes = [1, p.values.length];
  for (const v of p.values) pushVarint(bytes, Math.round(v * 100));
  let name = Array.from(new TextEncoder().encode(p.name.trim()));
  if (name.length > 40) {
    let end = 40;
    while (end > 0 && (name[end] & 0xc0) === 0x80) end--;
    name = name.slice(0, end);
  }
  bytes.push(name.length, ...name);
  const crc = crc16([letter.charCodeAt(0), ...bytes]);
  bytes.push(crc >> 8, crc & 0xff);
  return `LUNA-${letter}-${b64urlEncode(bytes)}`;
}

export function decodeShare(code: string): SharePayload {
  const clean = code.replace(/\s+/g, "");
  if (!/^luna-/i.test(clean)) throw new Error("This is not a Luna code (it should start with LUNA-).");
  const rest = clean.slice(5);
  const dash = rest.indexOf("-");
  if (dash !== 1) throw new Error("Unknown code type.");
  const letter = rest[0].toUpperCase();
  const kind = KINDS[letter];
  if (!kind) throw new Error("Unknown code type.");
  const bytes = b64urlDecode(rest.slice(2));
  if (bytes.length < 5) throw new Error("The code is too short.");
  const payload = bytes.slice(0, -2);
  const crc = (bytes[bytes.length - 2] << 8) | bytes[bytes.length - 1];
  if (crc16([letter.charCodeAt(0), ...payload]) !== crc) throw new Error("The code doesn't check out. Was it copied completely?");
  if (payload[0] !== 1) throw new Error("This code comes from a newer version of Luna.");
  const count = payload[1];
  const pos = { i: 2 };
  const values: number[] = [];
  for (let k = 0; k < count; k++) values.push(readVarint(payload, pos) / 100);
  const len = payload[pos.i++];
  if (len === undefined || pos.i + len !== payload.length) throw new Error("The code is cut short.");
  const name = new TextDecoder().decode(new Uint8Array(payload.slice(pos.i)));
  return { kind, name, values };
}
