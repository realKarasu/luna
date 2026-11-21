//! Which graphics card drives each monitor, read from DXGI: every adapter
//! lists its outputs, and each output carries the GDI device name
//! (`\\.\DISPLAY1`) that the monitor list uses too.

use crate::vibrance::Vendor;
use std::collections::HashMap;
use windows::Win32::Graphics::Dxgi::{CreateDXGIFactory1, IDXGIFactory1, DXGI_ADAPTER_FLAG_SOFTWARE};

#[derive(Clone, Debug, PartialEq)]
pub struct Gpu {
    pub name: String,
    pub vendor: Vendor,
}

fn utf16(buf: &[u16]) -> String {
    let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len]).trim().to_string()
}

/// Device names are compared without case (`\\.\DISPLAY1`).
pub fn key(device: &str) -> String {
    device.trim().to_ascii_uppercase()
}

/// Device name (upper case) to the graphics card behind it. Empty when DXGI
/// is not available; never fails.
pub fn detect() -> HashMap<String, Gpu> {
    let mut map = HashMap::new();
    let Ok(factory) = (unsafe { CreateDXGIFactory1::<IDXGIFactory1>() }) else {
        return map;
    };
    for i in 0..16u32 {
        let Ok(adapter) = (unsafe { factory.EnumAdapters1(i) }) else { break };
        let Ok(desc) = (unsafe { adapter.GetDesc1() }) else { continue };
        if desc.Flags & DXGI_ADAPTER_FLAG_SOFTWARE.0 as u32 != 0 {
            continue;
        }
        let gpu = Gpu { name: utf16(&desc.Description), vendor: Vendor::from_id(desc.VendorId) };
        for j in 0..16u32 {
            let Ok(output) = (unsafe { adapter.EnumOutputs(j) }) else { break };
            if let Ok(od) = unsafe { output.GetDesc() } {
                let name = utf16(&od.DeviceName);
                if !name.is_empty() {
                    map.entry(key(&name)).or_insert_with(|| gpu.clone());
                }
            }
        }
    }
    map
}
