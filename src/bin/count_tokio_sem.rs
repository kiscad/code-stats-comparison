use anyhow::Result;
use clap::Parser;
use futures::stream::{FuturesUnordered, StreamExt};
use glob::glob;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncReadExt;
use tokio::sync::mpsc;
use tokio::sync::Semaphore;

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
    let mut codes = 0;
    let mut blanks = 0;

    let buf = std::fs::read_to_string(path).unwrap();
    buf.lines().for_each(|line| {
        if line.trim().is_empty() {
            blanks += 1;
        } else {
            codes += 1;
        }
    });

    Ok(CodeStats {
        files: 1,
        blanks,
        codes,
    })
}

async fn count_file2(path: &Path) -> Result<CodeStats> {
    let mut codes = 0;
    let mut blanks = 0;

    let mut file = tokio::fs::File::open(path).await?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).await?;
    buf.lines().for_each(|line| {
        if line.trim().is_empty() {
            blanks += 1;
        } else {
            codes += 1;
        }
    });

    Ok(CodeStats {
        files: 1,
        blanks,
        codes,
    })
}

async fn count_dir(path: &Path, ext: &str) -> Result<CodeStats> {
    let paths = glob(&format!("{}**/*.{}", path.to_string_lossy(), ext))?;
    let paths = paths.filter_map(|p| p.ok());

    let mut futs = FuturesUnordered::new();
    let mut stats = CodeStats::new();

    for path in paths {
        let fut = tokio::spawn(async move { count_file(&path) });
        futs.push(fut);

        if futs.len() == 1000 {
            if let Some(Ok(Ok(s))) = futs.next().await {
                stats += s;
            }
        }
    }

    while let Some(Ok(Ok(s))) = futs.next().await {
        stats += s;
    }
    Ok(stats)
}

async fn count_dir2(path: &Path, ext: &str) -> Result<CodeStats> {
    let paths = glob(&format!("{}**/*.{}", path.to_string_lossy(), ext))?;
    let paths = paths.filter_map(|p| p.ok());

    let (tx, mut rx) = mpsc::channel(300);
    let sem = Arc::new(Semaphore::new(1000));

    for path in paths {
        let tx_ = tx.clone();
        let sem_clone = Arc::clone(&sem);
        tokio::spawn(async move {
            let aq = sem_clone.try_acquire();
            if let Ok(_guard) = aq {
                let count = count_file2(&path).await;
                if let Ok(s) = count {
                    tx_.send(s).await.unwrap();
                }
            }
        });
    }
    drop(tx);

    let mut res = CodeStats::new();
    while let Some(stats) = rx.recv().await {
        res += stats;
    }
    Ok(res)
}

fn print_result(res: HashMap<String, CodeStats>) {
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

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let exts: Vec<String> = args.exts;
    let path: PathBuf = args.path;

    let timer = Instant::now();
    let mut res = HashMap::new();
    let (tx, mut rx) = mpsc::channel(100);

    for ext in exts {
        let tx_ = tx.clone();
        let p_ = path.clone();
        tokio::spawn(async move {
            let stats = count_dir(&p_, &ext).await;
            if let Ok(s) = stats {
                tx_.send((ext, s)).await.unwrap();
            }
        });
    }
    drop(tx);

    while let Some((ext, stats)) = rx.recv().await {
        res.insert(ext, stats);
    }

    print_result(res);
    println!("Total time elapsed: {:?}", timer.elapsed());
}
