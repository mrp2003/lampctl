<div align="center">

# lampctl

**A fast TUI + CLI to control keyboard lighting over the open HID LampArray standard.**

[![CI](https://github.com/mrp2003/lampctl/actions/workflows/ci.yml/badge.svg)](https://github.com/mrp2003/lampctl/actions)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

</div>

> Lights up keyboards that ship **no working Linux driver** — by speaking the
> vendor-neutral **HID LampArray** protocol (Microsoft "Dynamic Lighting", HID usage
> page `0x59`) directly over `hidraw`.

`lampctl` was born from a real dead end: an **ASUS TUF Gaming A16 (FA608UP)** whose
keyboard backlight is dark on Linux because no driver implements it. It turns out the
keyboard is a perfectly standard **LampArray** device — so instead of waiting for a
vendor quirk, `lampctl` just talks the standard. The same code works on any LampArray
device, not just one laptop.

## Features

- 🎨 **Live preview** — your keyboard updates in real time as you scrub hue/brightness
- 🌈 **Modes** — static colour, rainbow cycle, breathing
- ⌨️ **TUI _and_ CLI** — a polished [ratatui](https://ratatui.rs) interface, plus
  `lampctl set 00e5ff` for scripts
- 📦 **Reusable library** — the [`lamparray`](crates/lamparray) crate is a tiny,
  dependency-light HID LampArray implementation you can use on its own
- 🪶 **No vendor SDK, no daemon required** — raw `hidraw`, one static binary

## Install

```sh
cargo install --path crates/lampctl
```

You'll need read/write access to the device's `hidraw` node. Add yourself to the
`input` group (and re-login), or install the provided udev rule.

## Usage

```sh
lampctl              # launch the TUI
lampctl set 00e5ff   # solid cyan
lampctl off          # lights out
lampctl list         # show detected LampArray devices
```

## How it works

A LampArray device exposes a handful of standard HID feature reports. `lampctl`:

1. **Discovers** devices by scanning `hidraw` report descriptors for usage page `0x59`.
2. Sends `LampArrayControl` (`AutonomousMode = 0`) to take control from the firmware.
3. Sends `LampRangeUpdate` to colour every lamp.

That's the whole trick — no reverse-engineering, no proprietary commands.

## Project layout

| Crate | What it is |
|---|---|
| [`lamparray`](crates/lamparray) | Library: discover + drive HID LampArray devices |
| [`lampctl`](crates/lampctl)   | Binary: the ratatui TUI + CLI |

## License

Dual-licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
