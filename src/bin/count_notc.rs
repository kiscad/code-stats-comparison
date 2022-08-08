use code_stats::Cli;
use code_stats::CodeStats;

use anyhow::Result;
use clap::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc::{self, Sender};

#[tokio::main(worker_threads = 2)]
async fn main() {
    let args = Cli::parse();
    let dir = Path::new(&args.dir);
    let types = Arc::new(args.types);
    let (tx, mut rx) = mpsc::channel(1000000);

    let timer = Instant::now();
    let res = tokio::spawn(async move {
        let mut res: HashMap<String, CodeStats> = HashMap::new();
        while let Some((ext, stats)) = rx.recv().await {
            let ent = res.entry(ext).or_default();
            *ent += stats;
        }
        res
    });

    count_dir(dir, types, tx).await.unwrap();

    match res.await {
        Ok(stats) => println!("{:?}", stats),
        Err(_) => println!("something wrong"),
    }

    println!("Total time used: {:?}", timer.elapsed());
}

#[async_recursion::async_recursion]
async fn count_dir(
    dir: &Path,
    valid_types: Arc<Vec<String>>,
    sender: Sender<(String, CodeStats)>,
) -> Result<()> {
    let paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|en| en.ok())
        .map(|en| en.path())
        .collect();
    let files = paths.iter().filter(|p| p.is_file());
    let dirs = paths.iter().filter(|p| p.is_dir() && !p.is_symlink());

    for f in files {
        if let Some(ext) = f.extension().and_then(std::ffi::OsStr::to_str) {
            if valid_types.iter().any(|t| t == ext) {
                let path = f.clone();
                let ext = ext.to_owned();
                let sender_ = sender.clone();
                tokio::spawn(async move { count_file(path, ext, sender_).await });
            }
        }
    }

    for d in dirs {
        let vtypes = valid_types.clone();
        let sender_ = sender.clone();
        let dir_ = d.clone();
        tokio::spawn(async move { count_dir(&dir_, vtypes, sender_).await });
    }

    Ok(())
}

async fn count_file(path: PathBuf, ext: String, sender: Sender<(String, CodeStats)>) {
    let mut codes = 0;
    let mut blanks = 0;

    if let Ok(buf) = std::fs::read_to_string(path) {
        buf.lines().for_each(|line| {
            if line.trim().is_empty() {
                blanks += 1;
            } else {
                codes += 1;
            }
        });
        let res = CodeStats {
            files: 1,
            blanks,
            codes,
        };
        let _ = sender.send((ext, res)).await;
    }
}
