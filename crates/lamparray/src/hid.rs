// Generic code

use crate::{LeU16, Rgb};

pub trait UsageId {
    const USAGE_ID: u8;
}

#[repr(C, packed)]
#[derive(Debug)]
pub struct Report<T> {
    pub report_id: u8,
    pub data: T,
}

#[repr(C, packed)]
#[derive(Default, Debug, Clone, Copy)]
pub struct Color {
    red: u8,
    green: u8,
    blue: u8,
    pub intensity: u8,
}

impl From<Rgb> for Color {
    fn from(value: Rgb) -> Self {
        let Rgb {
            r: red,
            g: green,
            b: blue,
        } = value;
        Color {
            red,
            green,
            blue,
            intensity: 255,
        }
    }
}

// Report implementation according to spec

#[repr(C, packed)]
#[derive(Default, Debug)]
pub struct LampArrayAttributesReport {
    pub lamp_count: LeU16,
    pub bb_w: u32,
    pub bb_h: u32,
    pub bb_d: u32,
    pub la_type: u32,
    pub min_upd_interval: u32,
}

impl UsageId for LampArrayAttributesReport {
    const USAGE_ID: u8 = 0x02;
}

#[repr(C, packed)]
#[derive(Default, Debug)]
pub struct LampAttributeRequestReport {
    lamp_id: LeU16,
}

impl UsageId for LampAttributeRequestReport {
    const USAGE_ID: u8 = 0x20;
}

#[repr(C, packed)]
#[derive(Default, Debug, Clone)]
pub struct LampAttributeResposeReport {
    pub lamp_id: LeU16,
    pos_x: u32,
    pos_y: u32,
    pos_z: u32,
    upd_latency: u32,
    purpose: u32,
    pub color: Color,
    is_programmable: u8,
    input_binding: u8,
}

impl UsageId for LampAttributeResposeReport {
    const USAGE_ID: u8 = 0x22;
}

#[repr(C, packed)]
#[derive(Default, Debug)]
pub struct LampMultiUpdateReport {
    pub lamp_count: u8,
    pub lamp_update_flags: u8,
    pub lamp_id: [LeU16; 8],
    pub colors: [Color; 8],
}

impl UsageId for LampMultiUpdateReport {
    const USAGE_ID: u8 = 0x50;
}

#[repr(C, packed)]
#[derive(Default, Debug)]
pub struct LampRangeUpdateReport {
    pub lamp_update_flags: u8,
    pub lamp_id_start: LeU16,
    pub lamp_id_end: LeU16,
    pub colors: Color,
}

impl UsageId for LampRangeUpdateReport {
    const USAGE_ID: u8 = 0x60;
}

#[repr(C, packed)]
#[derive(Default, Debug)]
pub struct LampArrayControlReport {
    pub autonomous_mode: u8,
}

impl UsageId for LampArrayControlReport {
    const USAGE_ID: u8 = 0x70;
}
