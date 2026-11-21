//! Short text share codes for presets and crosshairs.
//!
//! Format: `LUNA-<K>-<base64url>` where K is G (gamma preset), C (colour preset)
//! or X (crosshair). The base64 part holds:
//! `[version][value count][zigzag varint values x100...][name length][name utf-8][crc16]`.
//! The CRC (CCITT) covers the kind letter and every byte before it, so typos are caught.
//! `src/lib/sharecode.ts` is a port used only by the browser demo.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use serde::{Deserialize, Serialize};

const VERSION: u8 = 1;
const MAX_NAME_BYTES: usize = 40;
const MAX_VALUES: usize = 32;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SharePayload {
    /// "gamma", "color" or "crosshair".
    pub kind: String,
    pub name: String,
    /// Numbers, kept to two decimals.
    pub values: Vec<f64>,
}

fn kind_letter(kind: &str) -> Option<char> {
    match kind {
        "gamma" => Some('G'),
        "color" => Some('C'),
        "crosshair" => Some('X'),
        _ => None,
    }
}

fn letter_kind(c: char) -> Option<&'static str> {
    match c.to_ascii_uppercase() {
        'G' => Some("gamma"),
        'C' => Some("color"),
        'X' => Some("crosshair"),
        _ => None,
    }
}

pub fn crc16(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &b in data {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x1021 } else { crc << 1 };
        }
    }
    crc
}

fn push_varint(out: &mut Vec<u8>, v: i64) {
    let mut z = ((v << 1) ^ (v >> 63)) as u64;
    loop {
        let byte = (z & 0x7F) as u8;
        z >>= 7;
        if z == 0 {
            out.push(byte);
            break;
        }
        out.push(byte | 0x80);
    }
}

fn read_varint(data: &[u8], pos: &mut usize) -> Result<i64, String> {
    let mut z: u64 = 0;
    let mut shift = 0;
    loop {
        let b = *data.get(*pos).ok_or("The code is cut short.")?;
        *pos += 1;
        z |= ((b & 0x7F) as u64) << shift;
        if b & 0x80 == 0 {
            break;
        }
        shift += 7;
        if shift > 56 {
            return Err("The code has a broken number.".into());
        }
    }
    Ok(((z >> 1) as i64) ^ -((z & 1) as i64))
}

fn trim_name(name: &str) -> &str {
    let name = name.trim();
    if name.len() <= MAX_NAME_BYTES {
        return name;
    }
    let mut end = MAX_NAME_BYTES;
    while !name.is_char_boundary(end) {
        end -= 1;
    }
    &name[..end]
}

pub fn encode(p: &SharePayload) -> Result<String, String> {
    let letter = kind_letter(&p.kind).ok_or("Unknown share kind.")?;
    if p.values.len() > MAX_VALUES {
        return Err("Too many values to share.".into());
    }
    let mut bytes = vec![VERSION, p.values.len() as u8];
    for v in &p.values {
        if !v.is_finite() {
            return Err("A value is not a number.".into());
        }
        push_varint(&mut bytes, (v * 100.0).round() as i64);
    }
    let name = trim_name(&p.name);
    bytes.push(name.len() as u8);
    bytes.extend_from_slice(name.as_bytes());
    let mut signed = vec![letter as u8];
    signed.extend_from_slice(&bytes);
    let crc = crc16(&signed);
    bytes.extend_from_slice(&crc.to_be_bytes());
    Ok(format!("LUNA-{}-{}", letter, URL_SAFE_NO_PAD.encode(bytes)))
}

pub fn decode(code: &str) -> Result<SharePayload, String> {
    let clean: String = code.chars().filter(|c| !c.is_whitespace()).collect();
    let rest = clean
        .strip_prefix("LUNA-")
        .or_else(|| clean.strip_prefix("luna-"))
        .ok_or("This is not a Luna code (it should start with LUNA-).")?;
    let mut parts = rest.splitn(2, '-');
    let letter_part = parts.next().unwrap_or("");
    let body = parts.next().ok_or("The code is incomplete.")?;
    let letter = letter_part.chars().next().filter(|_| letter_part.len() == 1).ok_or("Unknown code type.")?;
    let kind = letter_kind(letter).ok_or("Unknown code type.")?;
    let bytes = URL_SAFE_NO_PAD.decode(body).map_err(|_| "The code has characters that don't belong in it.".to_string())?;
    if bytes.len() < 5 {
        return Err("The code is too short.".into());
    }
    let (payload, crc_bytes) = bytes.split_at(bytes.len() - 2);
    let mut signed = vec![letter.to_ascii_uppercase() as u8];
    signed.extend_from_slice(payload);
    if crc16(&signed) != u16::from_be_bytes([crc_bytes[0], crc_bytes[1]]) {
        return Err("The code doesn't check out. Was it copied completely?".into());
    }
    if payload[0] != VERSION {
        return Err("This code comes from a newer version of Luna.".into());
    }
    let count = payload[1] as usize;
    if count > MAX_VALUES {
        return Err("The code holds too many values.".into());
    }
    let mut pos = 2;
    let mut values = Vec::with_capacity(count);
    for _ in 0..count {
        values.push(read_varint(payload, &mut pos)? as f64 / 100.0);
    }
    let len = *payload.get(pos).ok_or("The code is cut short.")? as usize;
    pos += 1;
    let name_bytes = payload.get(pos..pos + len).ok_or("The code is cut short.")?;
    if pos + len != payload.len() {
        return Err("The code has extra data.".into());
    }
    let name = String::from_utf8(name_bytes.to_vec()).map_err(|_| "The name in the code is broken.".to_string())?;
    Ok(SharePayload { kind: kind.to_string(), name, values })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(kind: &str, name: &str, values: &[f64]) -> SharePayload {
        SharePayload { kind: kind.into(), name: name.into(), values: values.to_vec() }
    }

    #[test]
    fn roundtrip_gamma() {
        let a = p("gamma", "Shadow Lift", &[1.85, 3.0, 96.0, 100.0, 100.0, 100.0]);
        let code = encode(&a).unwrap();
        assert!(code.starts_with("LUNA-G-"));
        assert_eq!(decode(&code).unwrap(), a);
    }

    #[test]
    fn roundtrip_color_with_negatives() {
        let a = p("color", "Cool shift", &[-20.0, 115.0, 250.0]);
        let code = encode(&a).unwrap();
        assert!(code.starts_with("LUNA-C-"));
        assert_eq!(decode(&code).unwrap(), a);
    }

    #[test]
    fn roundtrip_crosshair_and_unicode_name() {
        let a = p("crosshair", "Étoile ☾", &[3.0, 1.0, 12.0, 8.0, 4.0, 2.0, 2.0, 1.0, 95.0]);
        let code = encode(&a).unwrap();
        assert!(code.starts_with("LUNA-X-"));
        assert_eq!(decode(&code).unwrap(), a);
    }

    #[test]
    fn values_keep_two_decimals() {
        let a = p("gamma", "x", &[1.234567]);
        assert_eq!(decode(&encode(&a).unwrap()).unwrap().values, vec![1.23]);
    }

    #[test]
    fn codes_are_short() {
        let code = encode(&p("color", "Neon", &[0.0, 240.0, 200.0])).unwrap();
        assert!(code.len() < 40, "{code}");
    }

    #[test]
    fn whitespace_is_ignored() {
        let a = p("color", "Neon", &[0.0, 240.0, 200.0]);
        let code = encode(&a).unwrap();
        let spaced = format!("  {} \n", code);
        assert_eq!(decode(&spaced).unwrap(), a);
    }

    #[test]
    fn tampered_code_is_rejected() {
        let code = encode(&p("gamma", "Lifted", &[1.5, 0.0, 100.0, 100.0, 100.0, 100.0])).unwrap();
        let mut chars: Vec<char> = code.chars().collect();
        let i = chars.len() - 4;
        chars[i] = if chars[i] == 'A' { 'B' } else { 'A' };
        let bad: String = chars.into_iter().collect();
        assert!(decode(&bad).is_err());
    }

    #[test]
    fn kind_letter_is_signed() {
        let code = encode(&p("gamma", "Lifted", &[1.5])).unwrap();
        let swapped = code.replacen("LUNA-G-", "LUNA-C-", 1);
        assert!(decode(&swapped).is_err());
    }

    #[test]
    fn wrong_prefix_and_garbage_are_rejected() {
        assert!(decode("HELLO").is_err());
        assert!(decode("LUNA-Q-AAAA").is_err());
        assert!(decode("LUNA-G-!!!!").is_err());
        assert!(decode("LUNA-G-").is_err());
        assert!(decode("").is_err());
    }

    #[test]
    fn long_names_are_trimmed_on_char_boundary() {
        let name = "é".repeat(50);
        let code = encode(&p("gamma", &name, &[1.0])).unwrap();
        let back = decode(&code).unwrap();
        assert!(back.name.len() <= MAX_NAME_BYTES);
        assert!(back.name.chars().all(|c| c == 'é'));
    }

    #[test]
    fn unknown_kind_cannot_be_encoded() {
        assert!(encode(&p("other", "x", &[])).is_err());
    }

    #[test]
    fn crc_known_value() {
        // CRC-16/CCITT-FALSE of "123456789" is 0x29B1.
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }
}
