use code_stats::Cli;
use code_stats::CodeStats;
use code_stats::TcRunner;

use anyhow::{anyhow, Result};
use clap::Parser;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() {
    let args = Cli::parse();
    let dir = Path::new(&args.dir);
    let types = Arc::new(args.types);

    let timer = Instant::now();
    let runner = Arc::new(TcRunner::new(1000));
    let res = count_dir(dir, types, runner).await;
    match res {
        Ok(stats) => println!("{:?}", stats),
        Err(_) => println!("something wrong"),
    }

    println!("Total time used: {:?}", timer.elapsed());
}

#[async_recursion::async_recursion]
async fn count_dir(
    dir: &Path,
    valid_types: Arc<Vec<String>>,
    runner: Arc<TcRunner>,
) -> Result<HashMap<String, CodeStats>> {
    let paths: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|en| en.ok())
        .map(|en| en.path())
        .collect();
    let files = paths.iter().filter(|p| p.is_file());
    let dirs = paths.iter().filter(|p| p.is_dir() && !p.is_symlink());

    let mut tasks = vec![];
    for f in files {
        if let Some(ext) = f.extension().and_then(std::ffi::OsStr::to_str) {
            if valid_types.iter().any(|t| t == ext) {
                let path = f.clone();
                let ext = ext.to_owned();
                tasks.push(
                    runner
                        .spawn(async move { count_file(path, ext).await })
                        .await,
                )
            }
        }
    }

    for d in dirs {
        let vtypes = valid_types.clone();
        let runner_ = runner.clone();
        let dir_ = d.clone();
        tasks.push(
            runner
                .spawn(async move { count_dir(&dir_, vtypes, runner_).await })
                .await,
        );
    }

    let mut res = HashMap::new();
    for t in tasks {
        if let Ok(s) = t.await.unwrap() {
            merge_hashmap_codestats(&mut res, s);
        }
    }
    Ok(res)
}

fn merge_hashmap_codestats(
    res: &mut HashMap<String, CodeStats>,
    other: HashMap<String, CodeStats>,
) {
    for (k, v) in other {
        match res.get_mut(&k) {
            Some(s) => {
                *s += v;
            }
            None => {
                res.insert(k, v);
            }
        }
    }
}

// use tokio::fs;

async fn count_file(path: PathBuf, ext: String) -> Result<HashMap<String, CodeStats>> {
    let mut codes = 0;
    let mut blanks = 0;

    // if let Ok(buf) = fs::read_to_string(path).await {
    if let Ok(buf) = std::fs::read_to_string(path) {
        buf.lines().for_each(|line| {
            if line.trim().is_empty() {
                blanks += 1;
            } else {
                codes += 1;
            }
        });
        let mut res = HashMap::new();
        res.insert(
            ext,
            CodeStats {
                files: 1,
                blanks,
                codes,
            },
        );
        Ok(res)
    } else {
        Err(anyhow!("cannot read file"))
    }
}
