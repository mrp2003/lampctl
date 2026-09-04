//! # lamparray
//!
//! A tiny, dependency-light Rust library for **HID LampArray** devices — the open
//! lighting standard behind Microsoft "Dynamic Lighting" (HID usage page `0x59`).
//!
//! Many modern keyboards (including a number of ASUS TUF/ROG laptops) expose their
//! backlight as a standard LampArray device but ship *no* working vendor driver on
//! Linux. This crate speaks the standard directly over `hidraw`, so it works without
//! any vendor-specific code.
//!
//! ```no_run
//! use lamparray::{LampArray, Rgb};
//! let mut kbd = LampArray::open_first()?;
//! kbd.set_all(Rgb::new(0, 0xE5, 0xFF))?; // cyan
//! # Ok::<(), std::io::Error>(())
//! ```
#![forbid(unsafe_op_in_unsafe_fn)]

use std::fmt::Debug;
use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use hidparser::parse_report_descriptor;
use hidparser::ReportField;

use crate::hid::{
    Color, LampArrayAttributesReport, LampArrayControlReport, LampAttributeRequestReport,
    LampAttributeResposeReport, LampMultiUpdateReport, LampRangeUpdateReport, UsageId,
};

mod hid;
mod ioctl;

#[derive(Default, Clone, Copy)]
pub struct LeU16([u8; 2]);

impl Debug for LeU16 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value: u16 = (*self).into();
        f.debug_tuple("LeU16").field(&value).finish()
    }
}

impl From<u16> for LeU16 {
    fn from(value: u16) -> Self {
        Self(value.to_le_bytes())
    }
}

impl From<[u8; 2]> for LeU16 {
    fn from(value: [u8; 2]) -> Self {
        Self(value)
    }
}

impl From<LeU16> for u16 {
    fn from(value: LeU16) -> Self {
        u16::from_le_bytes(value.0)
    }
}

impl LeU16 {
    fn into_u16(self) -> u16 {
        self.into()
    }
}

/// A 24-bit RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub const BLACK: Rgb = Rgb::new(0, 0, 0);
    pub const WHITE: Rgb = Rgb::new(0xFF, 0xFF, 0xFF);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// Parse `"RRGGBB"` or `"#RRGGBB"`.
    pub fn from_hex(s: &str) -> Option<Rgb> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        Some(Rgb::new(
            u8::from_str_radix(&s[0..2], 16).ok()?,
            u8::from_str_radix(&s[2..4], 16).ok()?,
            u8::from_str_radix(&s[4..6], 16).ok()?,
        ))
    }

    /// Scale brightness by `level` (0..=255).
    pub fn scaled(self, level: u8) -> Rgb {
        let f = |c: u8| ((c as u16 * level as u16) / 255) as u8;
        Rgb::new(f(self.r), f(self.g), f(self.b))
    }
}

// HID usage page for "Lighting And Illumination" (LampArray): `Usage Page (0x59)`.
const USAGE_PAGE_LIGHTING_ID: u8 = 0x59;
const USAGE_PAGE_LIGHTING: [u8; 2] = [0x05, USAGE_PAGE_LIGHTING_ID];

// Standard LampArray feature reports, as exposed by this device class.
const FLAG_UPDATE_COMPLETE: u8 = 0x01;

/// A discovered LampArray device on the system.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Path to the `/dev/hidrawN` node.
    pub node: PathBuf,
    /// Human-readable HID name, if the kernel exposes one.
    pub name: String,
    /// USB/HID vendor id.
    pub vendor_id: u16,
    /// USB/HID product id.
    pub product_id: u16,
    pub lamp_array_attributes_report_id: u8,
    pub lamp_attribute_request_report_id: u8,
    pub lamp_attribute_response_report_id: u8,
    pub lamp_multi_update_report_id: u8,
    pub lamp_range_update_report_id: u8,
    pub lamp_array_control_report_id: u8,
}

/// Find every `hidraw` device that advertises the LampArray usage page.
pub fn discover() -> io::Result<Vec<DeviceInfo>> {
    let mut out = Vec::new();
    for entry in fs::read_dir("/sys/class/hidraw")? {
        let entry = entry?;
        let dev_dir = entry.path().join("device");
        let desc = match fs::read(dev_dir.join("report_descriptor")) {
            Ok(bytes) => bytes,
            Err(_) => continue,
        };

        if !contains(&desc, &USAGE_PAGE_LIGHTING) {
            continue;
        };

        let Ok(collections) = parse_report_descriptor(&desc) else {
            continue;
        };

        let mut lamp_array_attributes_report_id = None;
        let mut lamp_attribute_request_report_id = None;
        let mut lamp_attribute_response_report_id = None;
        let mut lamp_multi_update_report_id = None;
        let mut lamp_range_update_report_id = None;
        let mut lamp_array_control_report_id = None;
        for feature in collections.features {
            if let &Some(ReportField::Variable(var)) = &feature.fields.first() {
                if var.usage.page() as u8 == USAGE_PAGE_LIGHTING_ID {
                    let Some(id) = feature.report_id else {
                        continue;
                    };
                    let usage = feature
                        .fields
                        .iter()
                        .filter_map(|f| match f {
                            ReportField::Variable(var) => var.member_of.get(1),
                            ReportField::Array(_) => None,
                            ReportField::Padding(_) => None,
                        })
                        .next()
                        .unwrap()
                        .usage
                        .id() as u8;
                    let id: u32 = id.into();
                    let id = Some(id as u8);
                    match usage {
                        LampArrayAttributesReport::USAGE_ID => lamp_array_attributes_report_id = id,
                        LampAttributeRequestReport::USAGE_ID => {
                            lamp_attribute_request_report_id = id
                        }
                        LampAttributeResposeReport::USAGE_ID => {
                            lamp_attribute_response_report_id = id
                        }
                        LampMultiUpdateReport::USAGE_ID => lamp_multi_update_report_id = id,
                        LampRangeUpdateReport::USAGE_ID => lamp_range_update_report_id = id,
                        LampArrayControlReport::USAGE_ID => lamp_array_control_report_id = id,
                        _ => (),
                    }
                }
            }
        }

        let uevent = fs::read_to_string(dev_dir.join("uevent")).unwrap_or_default();
        let name = uevent
            .lines()
            .find_map(|l| l.strip_prefix("HID_NAME=").map(str::to_owned))
            .unwrap_or_else(|| "LampArray device".to_string());
        let (vendor_id, product_id) = uevent
            .lines()
            .find_map(|l| l.strip_prefix("HID_ID="))
            .and_then(parse_hid_id)
            .unwrap_or((0, 0));

        fn rep_missing_err(name: &str) -> io::Error {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Missing report: {name}"),
            )
        }

        out.push(DeviceInfo {
            node: PathBuf::from("/dev").join(entry.file_name()),
            name,
            vendor_id,
            product_id,
            lamp_array_attributes_report_id: lamp_array_attributes_report_id
                .ok_or_else(|| rep_missing_err("lamp array attributes report"))?,
            lamp_attribute_request_report_id: lamp_attribute_request_report_id
                .ok_or_else(|| rep_missing_err("lamp attribute request report"))?,
            lamp_attribute_response_report_id: lamp_attribute_response_report_id
                .ok_or_else(|| rep_missing_err("lamp attribute response report"))?,
            lamp_multi_update_report_id: lamp_multi_update_report_id
                .ok_or_else(|| rep_missing_err("lamp multi update report"))?,
            lamp_range_update_report_id: lamp_range_update_report_id
                .ok_or_else(|| rep_missing_err("lamp range update report"))?,
            lamp_array_control_report_id: lamp_array_control_report_id
                .ok_or_else(|| rep_missing_err("lamp array control report"))?,
        });
    }
    Ok(out)
}

/// Parse a `HID_ID=bus:vendor:product` string (e.g. `0018:00000B05:000019B6`).
fn parse_hid_id(s: &str) -> Option<(u16, u16)> {
    let mut parts = s.trim().split(':');
    let _bus = parts.next()?;
    let vendor = u32::from_str_radix(parts.next()?.trim(), 16).ok()? as u16;
    let product = u32::from_str_radix(parts.next()?.trim(), 16).ok()? as u16;
    Some((vendor, product))
}

/// An open handle to a LampArray device.
pub struct LampArray {
    file: fs::File,
    info: DeviceInfo,
    lamp_count: LeU16,
    lamps: Vec<LampAttributeResposeReport>,
}

impl LampArray {
    /// Open a specific `/dev/hidrawN` node.
    pub fn open(info: DeviceInfo) -> io::Result<Self> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&info.node)?;
        let mut dev = LampArray {
            file,
            info,
            lamp_count: LeU16::default(),
            lamps: Vec::new(),
        };
        dev.query_lamp_count()?;
        dev.lamps = vec![LampAttributeResposeReport::default(); dev.lamp_count.into_u16() as usize];
        for _ in 0..dev.lamp_count.into_u16() {
            let info = dev.query_lamp_info()?;
            let idx = info.lamp_id.into_u16() as usize;
            dev.lamps[idx] = info;
        }

        Ok(dev)
    }

    /// Open the first LampArray device found via [`discover`].
    pub fn open_first() -> io::Result<Self> {
        let info = discover()?.into_iter().next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no HID LampArray device found")
        })?;
        Self::open(info)
    }

    /// Open the first LampArray device found via [`discover`].
    pub fn open_all() -> io::Result<Vec<Self>> {
        discover()?.into_iter().map(Self::open).collect()
    }

    /// Path of the underlying device node.
    pub fn node(&self) -> &Path {
        &self.info.node
    }

    /// Number of independently addressable lamps (1 == single-zone).
    pub fn lamp_count(&self) -> u16 {
        self.lamp_count.into()
    }

    fn query_lamp_count(&mut self) -> io::Result<u16> {
        let buf = ioctl::get_feature_typed::<hid::LampArrayAttributesReport>(
            self.file.as_raw_fd(),
            self.info.lamp_array_attributes_report_id,
        )?;
        self.lamp_count = buf.lamp_count;
        Ok(self.lamp_count.into())
    }

    pub fn query_lamp_info(&mut self) -> io::Result<LampAttributeResposeReport> {
        let buf = ioctl::get_feature_typed::<hid::LampAttributeResposeReport>(
            self.file.as_raw_fd(),
            self.info.lamp_attribute_response_report_id,
        )?;
        Ok(buf)
    }

    /// Toggle hardware "autonomous" mode. `false` hands lighting control to the host
    /// (i.e. takes it out of firmware / Windows Dynamic Lighting mode).
    pub fn set_autonomous(&mut self, on: bool) -> io::Result<()> {
        let buf = hid::LampArrayControlReport {
            autonomous_mode: on as u8,
        };
        ioctl::set_feature_typed(
            self.file.as_raw_fd(),
            self.info.lamp_array_control_report_id,
            buf,
        )?;
        Ok(())
    }

    pub fn set_multi(&mut self, lamp_id: [LeU16; 8], colors: [Color; 8]) -> io::Result<()> {
        let buf = LampMultiUpdateReport {
            lamp_count: 8,
            lamp_update_flags: FLAG_UPDATE_COMPLETE,
            lamp_id,
            colors,
        };

        ioctl::set_feature_typed(
            self.file.as_raw_fd(),
            self.info.lamp_multi_update_report_id,
            &buf,
        )?;
        Ok(())
    }

    /// Set a contiguous range of lamp ids to a single colour.
    pub fn set_range(&mut self, start: u16, end: u16, c: Rgb) -> io::Result<()> {
        let buf = LampRangeUpdateReport {
            lamp_update_flags: FLAG_UPDATE_COMPLETE,
            lamp_id_start: start.into(),
            lamp_id_end: end.into(),
            colors: c.into(),
        };

        ioctl::set_feature_typed(
            self.file.as_raw_fd(),
            self.info.lamp_range_update_report_id,
            buf,
        )?;
        Ok(())
    }

    /// Set every lamp to one color (takes host control first).
    pub fn set_all(&mut self, c: Rgb) -> io::Result<()> {
        self.set_autonomous(false)?;
        let end = self.lamp_count.into_u16().saturating_sub(1);
        self.set_range(0, end, c)
    }
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trip() {
        assert_eq!(Rgb::from_hex("#00E5FF"), Some(Rgb::new(0, 0xE5, 0xFF)));
        assert_eq!(Rgb::from_hex("ffffff"), Some(Rgb::WHITE));
        assert_eq!(Rgb::from_hex("nope"), None);
        assert_eq!(Rgb::from_hex("12345"), None);
    }

    #[test]
    fn scaling() {
        assert_eq!(Rgb::WHITE.scaled(0), Rgb::BLACK);
        assert_eq!(Rgb::WHITE.scaled(255), Rgb::WHITE);
        assert_eq!(Rgb::new(200, 100, 0).scaled(128).r, 100);
    }

    #[test]
    fn subslice_search() {
        assert!(contains(&[1, 2, 0x05, 0x59, 9], &USAGE_PAGE_LIGHTING));
        assert!(!contains(&[1, 2, 3], &USAGE_PAGE_LIGHTING));
    }

    #[test]
    fn hid_id_parsing() {
        assert_eq!(
            parse_hid_id("0018:00000B05:000019B6"),
            Some((0x0B05, 0x19B6))
        );
        assert_eq!(parse_hid_id("garbage"), None);
    }
}
