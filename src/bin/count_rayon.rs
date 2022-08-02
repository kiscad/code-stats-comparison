use anyhow::Result;
use clap::Parser;
use glob::glob;
use rayon::iter::ParallelBridge;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::Instant;

#[derive(Debug, Parser)]
struct Args {
    #[clap(short = 't')]
    exts: Vec<String>,
    #[clap(short = 'f')]
    path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct CodeStats {
    files: usize,
    blanks: usize,
    codes: usize,
}

impl CodeStats {
    fn new() -> Self {
        Self::default()
    }

    fn lines(&self) -> usize {
        self.blanks + self.codes
    }
}

impl std::ops::Add for CodeStats {
    type Output = CodeStats;
    fn add(self, rhs: Self) -> Self::Output {
        CodeStats {
            files: self.files + rhs.files,
            blanks: self.blanks + rhs.blanks,
            codes: self.codes + rhs.codes,
        }
    }
}

impl std::ops::AddAssign for CodeStats {
    fn add_assign(&mut self, rhs: Self) {
        self.files += rhs.files;
        self.blanks += rhs.blanks;
        self.codes += rhs.codes;
    }
}

fn count_file(path: &Path) -> Result<CodeStats> {
    let file = std::fs::File::open(path)?;
    let mut blanks = 0;
    let mut codes = 0;
    BufReader::new(file).lines().for_each(|line_res| {
        if let Ok(line) = line_res {
            if line.trim().is_empty() {
                blanks += 1;
            } else {
                codes += 1;
            }
        }
    });
    Ok(CodeStats {
        files: 1,
        blanks,
        codes,
    })
}

fn count_dir(path: &Path, ext: &str) -> Result<CodeStats> {
    let paths = glob(&format!("{}**/*.{}", path.to_string_lossy(), ext))?;
    let res = paths
        .par_bridge()
        .filter_map(|path| path.ok())
        .filter_map(|path| count_file(&path).ok())
        .reduce(CodeStats::new, |a, b| a + b);

    Ok(res)
}

fn print_result(res: HashMap<&String, CodeStats>) {
    println!(
        " {:<6} {:>12} {:>12} {:>12} {:>12}",
        "Ext.", "Files", "Lines", "Codes", "Blanks"
    );
    for (ext, stats) in res.iter() {
        println!(
            " {:<6} {:>12} {:>12} {:>12} {:>12}",
            ext,
            stats.files,
            stats.lines(),
            stats.codes,
            stats.blanks
        );
    }
}

fn main() {
    let args = Args::parse();
    let exts: Vec<String> = args.exts;
    let path: PathBuf = args.path;

    let timer = Instant::now();
    let res: HashMap<_, _> = exts
        .par_iter()
        .zip(exts.par_iter().map(|ext| count_dir(&path, ext)))
        .filter(|(_, stats)| stats.is_ok())
        .map(|(ext, stats)| (ext, stats.ok().unwrap()))
        .collect();

    print_result(res);
    println!("Total time elapsed: {:?}", timer.elapsed());
}
