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

use std::fs;
use std::io;
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

mod ioctl;

/// A 24-bit RGB colour.
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
const USAGE_PAGE_LIGHTING: [u8; 2] = [0x05, 0x59];

// Standard LampArray feature reports, as exposed by this device class.
const REPORT_ATTRIBUTES: u8 = 0x41; // GET: LampArrayAttributes -> LampCount
const REPORT_RANGE_UPDATE: u8 = 0x45; // SET: LampRangeUpdate
const REPORT_CONTROL: u8 = 0x46; // SET: LampArrayControl -> AutonomousMode
const FLAG_UPDATE_COMPLETE: u8 = 0x01;

/// A discovered LampArray device on the system.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Path to the `/dev/hidrawN` node.
    pub node: PathBuf,
    /// Human-readable HID name, if the kernel exposes one.
    pub name: String,
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
        }
        let name = fs::read_to_string(dev_dir.join("uevent"))
            .ok()
            .and_then(|u| {
                u.lines()
                    .find_map(|l| l.strip_prefix("HID_NAME=").map(str::to_owned))
            })
            .unwrap_or_else(|| "LampArray device".to_string());
        out.push(DeviceInfo {
            node: PathBuf::from("/dev").join(entry.file_name()),
            name,
        });
    }
    Ok(out)
}

/// An open handle to a LampArray device.
pub struct LampArray {
    file: fs::File,
    node: PathBuf,
    lamp_count: u16,
}

impl LampArray {
    /// Open a specific `/dev/hidrawN` node.
    pub fn open(node: &Path) -> io::Result<Self> {
        let file = fs::OpenOptions::new().read(true).write(true).open(node)?;
        let mut dev = LampArray {
            file,
            node: node.to_path_buf(),
            lamp_count: 1,
        };
        dev.lamp_count = dev.query_lamp_count().unwrap_or(1).max(1);
        Ok(dev)
    }

    /// Open the first LampArray device found via [`discover`].
    pub fn open_first() -> io::Result<Self> {
        let info = discover()?.into_iter().next().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "no HID LampArray device found")
        })?;
        Self::open(&info.node)
    }

    /// Path of the underlying device node.
    pub fn node(&self) -> &Path {
        &self.node
    }

    /// Number of independently addressable lamps (1 == single-zone).
    pub fn lamp_count(&self) -> u16 {
        self.lamp_count
    }

    fn query_lamp_count(&mut self) -> io::Result<u16> {
        let mut buf = [0u8; 8];
        buf[0] = REPORT_ATTRIBUTES;
        ioctl::get_feature(self.file.as_raw_fd(), &mut buf)?;
        Ok(u16::from_le_bytes([buf[1], buf[2]]))
    }

    /// Toggle hardware "autonomous" mode. `false` hands lighting control to the host
    /// (i.e. takes it out of firmware / Windows Dynamic Lighting mode).
    pub fn set_autonomous(&mut self, on: bool) -> io::Result<()> {
        ioctl::set_feature(self.file.as_raw_fd(), &[REPORT_CONTROL, on as u8])
    }

    /// Set a contiguous range of lamp ids to a single colour.
    pub fn set_range(&mut self, start: u16, end: u16, c: Rgb) -> io::Result<()> {
        let mut buf = [0u8; 10];
        buf[0] = REPORT_RANGE_UPDATE;
        buf[1] = FLAG_UPDATE_COMPLETE;
        buf[2..4].copy_from_slice(&start.to_le_bytes());
        buf[4..6].copy_from_slice(&end.to_le_bytes());
        buf[6] = c.r;
        buf[7] = c.g;
        buf[8] = c.b;
        buf[9] = 0xFF; // intensity channel
        ioctl::set_feature(self.file.as_raw_fd(), &buf)
    }

    /// Set every lamp to one colour (takes host control first).
    pub fn set_all(&mut self, c: Rgb) -> io::Result<()> {
        self.set_autonomous(false)?;
        let end = self.lamp_count.saturating_sub(1);
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
}
