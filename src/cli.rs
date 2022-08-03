use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Parser)]
pub struct Cli {
    #[clap(short = 't')]
    pub types: Vec<String>,
    #[clap(short = 'f')]
    pub dir: PathBuf,
}
