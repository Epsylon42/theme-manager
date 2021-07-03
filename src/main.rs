use std::path::PathBuf;

use argh::FromArgs;

pub mod install;
pub mod themes;
pub mod error;
pub mod utils;
pub mod hooks;
pub mod manager;

mod prelude {
    pub use crate::error::{Error, ErrorExt, ResultExt};
    pub use crate::utils;

    pub use log::{trace, warn, error};
}

use manager::ThemeManager;
use prelude::*;

#[derive(FromArgs)]
///
struct Args {
    #[argh(option)]
    /// dir
    dir: Option<PathBuf>,
    #[argh(subcommand)]
    command: Subcommand,
}

#[derive(FromArgs)]
#[argh(subcommand)]
enum Subcommand {
    Install(InstallCommand),
    Display(DisplayCommand),
}

#[derive(FromArgs)]
#[argh(subcommand, name = "install")]
///
struct InstallCommand {
    #[argh(positional)]
    theme_name: String
}

#[derive(FromArgs)]
#[argh(subcommand, name = "display")]
///
struct DisplayCommand {

}

fn main() -> Result<(), String> {
    run().map_err(|e| e.to_string())
}

fn run() -> Result<(), Error> {
    env_logger::init();

    let args: Args = argh::from_env();

    let dir = match args.dir {
        Some(dir) => dir,
        None => std::env::var_os("THEME_MANAGER_DIR")
            .map(PathBuf::from)
            .ok_or(Error::NoDir)?,
    };

    let manager = ThemeManager::read_from_dir(&dir)?;

    match args.command {
        Subcommand::Install(InstallCommand {
            theme_name,
        }) => {
            manager.install_theme(&theme_name);
        }

        Subcommand::Display(DisplayCommand {}) => {
            dbg!(manager);
        }
    }

    Ok(())
}
