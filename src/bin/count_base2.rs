use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use clap::Parser;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(short = 't')]
    types: Vec<String>,
    #[clap(short = 'f')]
    dirs: Vec<PathBuf>,
}

#[derive(Debug, Clone, Default)]
struct CodeStats {
    files: usize,
    blanks: usize,
    codes: usize,
}

impl std::ops::AddAssign for CodeStats {
    fn add_assign(&mut self, rhs: Self) {
        self.files += rhs.files;
        self.blanks += rhs.blanks;
        self.codes += rhs.codes;
    }
}

// type StatKey = (PathBuf, String);

fn main() {
    let args = Cli::parse();
    let types = Arc::new(args.types.clone());
    let timer = std::time::Instant::now();

    let (sender, receiver) = mpsc::channel();

    let mut thread_handles = vec![];
    for dir in args.dirs {
        let sender_ = sender.clone();
        let types_ = types.clone();
        thread_handles.push(thread::spawn(move || find_files(&dir, types_, sender_)));
    }
    drop(sender);
    for handle in thread_handles {
        handle.join().unwrap().unwrap();
    }

    let mut stats_tot = HashMap::new();
    for (type_, stats) in receiver {
        let s = stats_tot.entry(type_).or_insert_with(CodeStats::default);
        *s += stats;
    }
    println!("{:?}", stats_tot);
    println!("Total used time: {:?}", timer.elapsed());
}

fn count_lines(file_path: &Path, ext: String, sender: Sender<(String, CodeStats)>) {
    let mut codes = 0;
    let mut blanks = 0;
    let buf = std::fs::read_to_string(file_path);
    if let Ok(content) = buf {
        content.lines().for_each(|line| {
            if line.trim().is_empty() {
                blanks += 1;
            } else {
                codes += 1;
            }
        });
        let stats = CodeStats {
            files: 1,
            blanks,
            codes,
        };
        if sender.send((ext, stats)).is_err() {
            println!("Failed to send codestats of file: {:?}", file_path);
        }
    }
}

fn find_files(
    dir: &Path,
    types: Arc<Vec<String>>,
    sender: Sender<(String, CodeStats)>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && !path.is_symlink() {
            find_files(&path, types.clone(), sender.clone())?;
        } else {
            let ext = path.extension().and_then(OsStr::to_str);
            if let Some(ext) = ext {
                if types.iter().any(|t| t == ext) {
                    count_lines(&path, ext.to_owned(), sender.clone());
                }
            }
        }
    }
    Ok(())
}
