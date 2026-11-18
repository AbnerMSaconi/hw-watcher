// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Read-only BAR0 (`resource0`) sensor for verified GPUs (GA102 / RTX 3090).

#![allow(unsafe_code)]

use std::fs::OpenOptions;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

use hw_watcher_core::{Bar0Source, decode_bar0_hotspot_c, decode_bar0_vram_c};
use memmap2::{Mmap, MmapOptions};

const PCI_ID_GA102: u32 = 0x2204_10DE;
const OFFSET_VRAM: usize = 0x0000_E2A8;
const OFFSET_HOTSPOT: usize = 0x0002_046C;

/// Reads VRAM junction and hotspot temperatures from a verified GPU's PCI BAR0.
pub struct Bar0TempSensor {
    mmap: Mmap,
}

impl Bar0TempSensor {
    /// Opens `/sys/bus/pci/devices/<pci>/resource0` read-only (`PROT_READ`).
    ///
    /// Returns `None` for unverified PCI IDs or when permissions/hardware deny access.
    pub fn open(pci: &str, pci_device_id: u32) -> Option<Self> {
        if pci_device_id != PCI_ID_GA102 || pci.is_empty() {
            return None;
        }
        let path = PathBuf::from(format!("/sys/bus/pci/devices/{pci}/resource0"));
        let file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_SYNC)
            .open(path)
            .ok()?;
        // SAFETY: Read-only mapping (`PROT_READ`) of PCI BAR0; all reads go through bounds-checked
        // `read_u32_volatile`.
        let mmap = unsafe { MmapOptions::new().map(&file) }.ok()?;
        if mmap.len() < OFFSET_HOTSPOT + 4 {
            return None;
        }
        Some(Self { mmap })
    }
}

impl Bar0Source for Bar0TempSensor {
    fn read_temps(&self) -> (Option<f64>, Option<f64>) {
        let vram = read_u32_volatile(&self.mmap, OFFSET_VRAM).and_then(decode_bar0_vram_c);
        let hotspot = read_u32_volatile(&self.mmap, OFFSET_HOTSPOT).and_then(decode_bar0_hotspot_c);
        (vram, hotspot)
    }
}

#[allow(clippy::cast_ptr_alignment)]
fn read_u32_volatile(mmap: &Mmap, offset: usize) -> Option<u32> {
    if !offset.is_multiple_of(4) || offset.checked_add(4)? > mmap.len() {
        return None;
    }
    // SAFETY: `mmap` is page-aligned, `offset` is 4-byte aligned, and `offset + 4 <= mmap.len()`.
    Some(unsafe {
        let ptr = mmap.as_ptr().add(offset).cast::<u32>();
        u32::from_le(ptr.read_volatile())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_unverified_pci_device_id() {
        assert!(Bar0TempSensor::open("0000:2b:00.0", 0x1C03_10DE).is_none());
        assert!(Bar0TempSensor::open("", PCI_ID_GA102).is_none());
    }
}
