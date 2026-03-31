//! Cosmic Garden — a multi-threaded MUD engine.
use std::{ops::Deref, time::Duration};

use clap::Parser;

mod life_thread;
use life_thread::life_thread;
mod io; use io::*;
mod io_thread; use io_thread::io_thread;
mod identity;

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Cosmic Garden MUD Engine.",
//    after_help = ""
)]
struct Cli {
    #[arg(short, long, default_value = "8080")] port: u32,
    #[arg(long, default_value = "0.0.0.0")] host_listen_addr: String,
    #[arg(long, default_value = "cosmic-garden")] world: String,
    #[arg(long, env = "COSMIC_GARDEN_DATA", default_value = "data")] data_path: String,
    #[arg(long)] bootstrap_url: Option<String>,
    #[arg(long)] autosave_queue_interval: Option<u64>,
}

/// The main culprit of many things main…
#[tokio::main]
async fn main() {
    // some const to deal with [World]-specific choices that aren't present for a reason or other…
    const GREETING: &'static str = "Welcome to Cosmic Garden!";
    const PROMPT_LOGIN: &'static str = "Login: ";
    const PROMPT_PWD1: &'static str = "Password: ";
    const PROMPT_PWDV: &'static str = "Re-type same pwd: ";
    const WELCOME_BACK: &'static str = "Welcome back!";
    const WELCOME_NEW: &'static str = "May your adventures be prosperous!";

    let _ = env_logger::try_init();
    let args = Cli::parse();
    let _ = DATA.set(args.data_path);

    tokio::spawn(life_thread());
    tokio::spawn(io_thread());

    tokio::time::sleep(Duration::from_secs(5)).await;
}
