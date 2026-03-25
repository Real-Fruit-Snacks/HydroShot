use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "hydroshot", about = "Screenshot capture and annotation tool")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Capture a screenshot
    Capture {
        /// Copy full screen to clipboard (non-interactive)
        #[arg(long)]
        clipboard: bool,

        /// Save full screen to file path (non-interactive)
        #[arg(long)]
        save: Option<String>,

        /// Delay in seconds before capturing
        #[arg(long, default_value_t = 0)]
        delay: u64,
    },
}
