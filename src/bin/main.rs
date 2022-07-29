use std::path::Path;
use std::{
    sync::{Arc,Mutex},
    time::Instant,
    process,
    env,
};
use std::collections::{HashMap};
use tokio::task::{self, JoinHandle};
use rust_lines::config::{Config, RunMode};
use rust_lines::log::log;
use rust_lines::statlines::{LinesStat, count_files, count_files_sync};

fn main() {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .max_blocking_threads(4)
        // .thread_stack_size(1024 * 1024)
        .build()
        .unwrap();
    runtime.block_on(
async {
    let config: Config = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });
    println!("Config from arguments: {:?}", config);
    let mut stats: HashMap<String, Arc<Mutex<HashMap<String, LinesStat>>>> = HashMap::new();
    let mut file_count_stats: HashMap<String, (u32, u32)> = HashMap::new();
    let mut handles: HashMap<String, JoinHandle<_>> = HashMap::new();
    //let types = vec!["rs", "c", "cc", "cpp", "cxx", "h", "hpp", "hxx", "java", "py", "kt", "js", "lua"];
    let types: Arc<Box<Vec<String>>> = Arc::new(Box::new(config.types));
    let before = Instant::now();
    let mut idx: u32 = 0;
    for dir_name in config.dirs {
        let dir = Path::new(dir_name.as_str());
        if !dir.is_dir() {
            continue;
        }
        idx += 1u32;
        let languages: Arc<Mutex<HashMap<String, LinesStat>>> = Arc::new(Mutex::new(HashMap::new()));
        let map = Arc::clone(&languages);
        let dirname_ = String::from("dir".to_owned() + idx.to_string().as_str() + &":".to_owned() + dir_name.as_str());
        stats.insert(dirname_, map);
        let map_ = Arc::clone(&languages);
        let types_ = Arc::clone(&types);

        // let total_files;
        // let matched_files;
        match config.run_mode {
            RunMode::MultipleThread => {
                let dir_parallel = Arc::new(Box::new(false));
                let dirname_ = String::from(dir_name.as_str());
                handles.insert(dirname_, tokio::spawn(task::block_in_place(|| {
                    count_files(dir.into(), types_, map_, dir_parallel)
                })));
            },
            RunMode::MultipleThread4Dir => {
                let dir_parallel = Arc::new(Box::new(true));
                let dirname_ = String::from(dir_name.as_str());
                handles.insert(dirname_, tokio::spawn(task::block_in_place(|| {
                    count_files(dir.into(), types_, map_, dir_parallel)
                })));
            },
            RunMode::SingleThread => {
                let dirname_ = String::from(dir_name.as_str());
                let (total_files, matched_files) = count_files_sync(dir.into(), types_, map_).unwrap();
                file_count_stats.insert(dirname_, (total_files, matched_files));
            }
        }
    }
    for (dir_name, h) in handles {
        let (total_files, matched_files)= h.await.unwrap().unwrap();
        file_count_stats.insert(dir_name, (total_files, matched_files));
    }

    log("print the lines statistics result:");
    println!("     directory                     ext      files_count    nonblank_line       blank_line");
    //              /home/oss/aosp                 c            43444          2005850         14541914
    let mut total_files = 0;
    let mut total_blank_lines = 0;
    let mut total_nonblank_lines = 0;    
    for (dir_name, languages) in stats {
        let map = languages.lock().unwrap();
        let mut sub_total_files = 0;
        let mut sub_total_blank_lines = 0;
        let mut sub_total_nonblank_lines = 0;    
        for (lang, lines) in map.iter(){
            println!("{dir:>width$}  {lang:>width1$}  {files_count:>width2$}  {nonblank_line:>width3$}  {blank_line:>width4$}", 
            dir = dir_name.as_str(), width = 20, lang = lang, width1 = 16, files_count = lines.files_count, width2 = 15, 
            nonblank_line = lines.nonblank_lines_count, width3 = 15, blank_line = lines.blank_lines_count, width4 = 15);
            sub_total_files += lines.files_count;
            sub_total_blank_lines += lines.blank_lines_count;
            sub_total_nonblank_lines += lines.nonblank_lines_count;
        }
        println!("   Sub Total                            {total_files:>width2$}  {nonblank_line:>width3$}  {blank_line:>width4$}", 
            total_files = sub_total_files, width2 = 15, 
            nonblank_line = sub_total_nonblank_lines, width3 = 15, blank_line = sub_total_blank_lines, width4 = 15);
        println!("");
        total_files += sub_total_files;
        total_blank_lines += sub_total_blank_lines;
        total_nonblank_lines += sub_total_nonblank_lines;
    }
    println!("       Total                            {total_files:>width2$}  {nonblank_line:>width3$}  {blank_line:>width4$}", 
    total_files = total_files, width2 = 15, 
    nonblank_line = total_nonblank_lines, width3 = 15, blank_line = total_blank_lines, width4 = 15);

    let elapsed = before.elapsed();
    println!("{:?} total", elapsed,);  
})
}
