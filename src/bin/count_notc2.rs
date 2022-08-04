use code_stats::Cli;
use code_stats::CodeStats;

use clap::Parser;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use tokio::sync::mpsc::{self, Receiver, Sender};

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let dir = args.dir.clone();
    let types = args.types;

    let (tx, rx) = mpsc::channel(100000);
    let timer = Instant::now();

    // start the task manager
    let handle = tokio::spawn(async move { task_manager(rx).await });
    // scan the folder recursively
    tokio::spawn(async move { scan_dir(Path::new(&dir), &types, tx).await });

    let res = handle.await.unwrap();
    println!("Code stats Result: {:#?}", res);
    println!("Total time used: {:?}", timer.elapsed());
}

async fn task_manager(mut rcvr: Receiver<(PathBuf, String)>) -> HashMap<String, CodeStats> {
    let (tx, mut rx) = mpsc::channel(10000);

    let res = tokio::spawn(async move {
        let mut res: HashMap<String, CodeStats> = HashMap::new();
        while let Some((ext, stats)) = rx.recv().await {
            let ent = res.entry(ext).or_default();
            *ent += stats;
        }
        res
    });

    while let Some((path, ext)) = rcvr.recv().await {
        let tx_ = tx.clone();
        tokio::spawn(async move { count_file(path, ext, tx_).await });
    }
    drop(tx);

    res.await.unwrap()
}

#[async_recursion::async_recursion]
async fn scan_dir(
    dir: &Path,
    types: &Vec<String>,
    sender: Sender<(PathBuf, String)>,
) -> std::io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() && !path.is_symlink() {
            scan_dir(&path, types, sender.clone()).await?;
        } else {
            let ext = path.extension().and_then(OsStr::to_str);
            if let Some(ext) = ext {
                if types.iter().any(|t| t == ext) {
                    let ext = ext.to_owned();
                    let _ = sender.send((path, ext)).await;
                }
            }
        }
    }
    Ok(())
}

async fn count_file(path: PathBuf, ext: String, sender: Sender<(String, CodeStats)>) {
    let mut codes = 0;
    let mut blanks = 0;
    if let Ok(buf) = fs::read_to_string(path) {
        buf.lines().for_each(|line| {
            if line.trim().is_empty() {
                blanks += 1;
            } else {
                codes += 1;
            }
        });
        let stats = CodeStats {
            files: 1,
            codes,
            blanks,
        };
        let _ = sender.send((ext, stats)).await;
    }
}
