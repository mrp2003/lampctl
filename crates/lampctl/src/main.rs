//! lampctl — a TUI + CLI for HID LampArray lighting.
use clap::{Parser, Subcommand};
use lamparray::{LampArray, Rgb};

mod app;
mod color;

#[derive(Parser)]
#[command(
    name = "lampctl",
    version,
    about = "Control HID LampArray keyboard lighting (TUI + CLI)"
)]
struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Set every lamp to a solid colour, e.g. `lampctl set 00e5ff`.
    Set {
        /// Colour as RRGGBB hex.
        hex: String,
    },
    /// Turn the lighting off.
    Off,
    /// List detected LampArray devices.
    List,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        Some(Cmd::Set { hex }) => {
            let c = Rgb::from_hex(&hex)
                .ok_or_else(|| anyhow::anyhow!("invalid colour '{hex}' (want RRGGBB)"))?;
            open()?.set_all(c)?;
        }
        Some(Cmd::Off) => open()?.set_all(Rgb::BLACK)?,
        Some(Cmd::List) => {
            let devices = lamparray::discover()?;
            if devices.is_empty() {
                println!("no HID LampArray devices found");
            }
            for d in devices {
                println!("{:<14} {}", d.node.display(), d.name);
            }
        }
        None => app::run()?,
    }
    Ok(())
}

fn open() -> anyhow::Result<LampArray> {
    LampArray::open_first().map_err(|e| {
        anyhow::anyhow!(
            "could not open a LampArray device: {e}\n\
             hint: make sure your user is in the 'input' group (newly added groups need a re-login)."
        )
    })
}
