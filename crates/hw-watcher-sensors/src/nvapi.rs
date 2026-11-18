// SPDX-License-Identifier: GPL-3.0-only
// Copyright (C) 2026 AbnerMSaconi

//! Optional read-only GPU core voltage reader via `libnvidia-api.so.1` (NVAPI).

#![allow(unsafe_code)]

use std::ffi::c_void;

use hw_watcher_core::VoltageSource;
use libloading::{Library, Symbol};

const LIB_NVAPI: &str = "libnvidia-api.so.1";
const ID_INITIALIZE: u32 = 0x0150_E828;
const ID_ENUM_GPUS: u32 = 0xE5AC_921F;
const ID_BUS_ID: u32 = 0x1BE0_B8E5;
const ID_VOLTAGE: u32 = 0x465F_9BCF;

const SIZE_VOLTAGE: usize = 76;
const OFFSET_UV: usize = 0x28;
const VERSION_VOLTAGE: u32 = (SIZE_VOLTAGE as u32) | (1 << 16);
const MAX_GPUS: usize = 64;
const UV_PER_VOLT: f64 = 1_000_000.0;

type QueryInterfaceFn = unsafe extern "C" fn(u32) -> *const c_void;
type InitFn = unsafe extern "C" fn() -> i32;
type EnumGpusFn = unsafe extern "C" fn(*mut *mut c_void, *mut u32) -> i32;
type BusIdFn = unsafe extern "C" fn(*mut c_void, *mut u32) -> i32;
type VoltageFn = unsafe extern "C" fn(*mut c_void, *mut u8) -> i32;

/// Reads GPU core voltage from NVAPI if `libnvidia-api.so.1` is present and accessible.
pub struct NvApiVoltageSensor {
    _lib: Library,
    gpu: *mut c_void,
    voltage_fn: VoltageFn,
}

// SAFETY: `gpu` is an opaque handle tied to `_lib` (dropped together) and read-only calls are
// serialized through `&self` / single-owner sampler thread.
unsafe impl Send for NvApiVoltageSensor {}

impl NvApiVoltageSensor {
    /// Opens NVAPI and matches the GPU at `pci` (e.g. `"0000:2b:00.0"`), returning `None` on failure.
    pub fn open(pci: &str) -> Option<Self> {
        let bus = parse_pci_bus(pci);
        // SAFETY: Loads NVIDIA's userspace NVAPI library and queries standard C entry points.
        unsafe {
            let lib = Library::new(LIB_NVAPI).ok()?;
            let sym: Symbol<'_, QueryInterfaceFn> = lib.get(b"nvapi_QueryInterface\0").ok()?;
            let qi: QueryInterfaceFn = *sym;

            let init: InitFn = query_fn(qi, ID_INITIALIZE)?;
            if init() != 0 {
                return None;
            }
            let enum_gpus: EnumGpusFn = query_fn(qi, ID_ENUM_GPUS)?;
            let mut handles = [std::ptr::null_mut::<c_void>(); MAX_GPUS];
            let mut count: u32 = 0;
            if enum_gpus(handles.as_mut_ptr(), &raw mut count) != 0 || count == 0 {
                return None;
            }
            let gpu = select_handle(qi, &handles[..count as usize], bus)?;
            let voltage_fn: VoltageFn = query_fn(qi, ID_VOLTAGE)?;
            let sensor = Self {
                _lib: lib,
                gpu,
                voltage_fn,
            };
            sensor.read_voltage_v()?;
            Some(sensor)
        }
    }
}

impl VoltageSource for NvApiVoltageSensor {
    fn read_voltage_v(&self) -> Option<f64> {
        let mut buf = [0u8; SIZE_VOLTAGE];
        buf[0..4].copy_from_slice(&VERSION_VOLTAGE.to_le_bytes());
        // SAFETY: `self.gpu` is a valid physical GPU handle and `buf` matches NVAPI `ClientVoltRailsGetStatus` v1 (76 bytes).
        let status = unsafe { (self.voltage_fn)(self.gpu, buf.as_mut_ptr()) };
        if status != 0 {
            return None;
        }
        let bytes: [u8; 4] = buf.get(OFFSET_UV..OFFSET_UV + 4)?.try_into().ok()?;
        Some(f64::from(u32::from_le_bytes(bytes)) / UV_PER_VOLT)
    }
}

unsafe fn query_fn<F: Copy>(qi: QueryInterfaceFn, id: u32) -> Option<F> {
    // SAFETY: Caller guarantees `F` matches the C function pointer signature for `id`.
    unsafe {
        let ptr = qi(id);
        if ptr.is_null() {
            return None;
        }
        Some(std::mem::transmute_copy(&ptr))
    }
}

unsafe fn select_handle(
    qi: QueryInterfaceFn,
    handles: &[*mut c_void],
    wanted_bus: Option<u32>,
) -> Option<*mut c_void> {
    let first = handles.first().copied().filter(|p| !p.is_null())?;
    let Some(target) = wanted_bus else {
        return (handles.len() == 1).then_some(first);
    };
    // SAFETY: `ID_BUS_ID` writes a single `u32` bus number for a valid physical GPU handle.
    let Some(get_bus): Option<BusIdFn> = (unsafe { query_fn(qi, ID_BUS_ID) }) else {
        return (handles.len() == 1).then_some(first);
    };
    for &handle in handles {
        if handle.is_null() {
            continue;
        }
        let mut bus: u32 = 0;
        if unsafe { get_bus(handle, &raw mut bus) } == 0 && bus == target {
            return Some(handle);
        }
    }
    None
}

fn parse_pci_bus(pci: &str) -> Option<u32> {
    let mut parts = pci.split(':');
    let _domain = parts.next()?;
    let bus = parts.next()?;
    let _dev = parts.next()?;
    u32::from_str_radix(bus, 16).ok()
}
