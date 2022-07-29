use std::path::Path;
use std::{
    sync::{Arc},
    time::Instant,
    process,
    env,
};
//use std::collections::{HashMap};
use rayon::prelude::*;

use rust_lines::config::{Config};
use rust_lines::log::log;
use rust_lines::statlines::{scan_dir};

fn main() {
    let config: Config = Config::new(env::args()).unwrap_or_else(|err| {
        println!("Problem parsing arguments: {}", err);
        process::exit(1);
    });
    println!("Config from arguments: {:?}", config);
    //let types = vec!["rs", "c", "cc", "cpp", "cxx", "h", "hpp", "hxx", "java", "py", "kt", "js", "lua"];
    let types: Arc<Box<Vec<String>>> = Arc::new(Box::new(config.types));
    let before = Instant::now();

    let stats = config.dirs.par_iter()
        .map(|dir_name| {
            let dir = Path::new(dir_name.as_str());
            let types_ = Arc::clone(&types);
            vec![(dir_name, scan_dir(&dir, types_))]
        })
        .reduce(|| std::vec::Vec::new(), |mut a, mut b| {
            a.append(&mut b);
            a
        });

    log("print the lines statistics result:");
    println!("     directory                     ext      files_count    nonblank_line       blank_line");
    //              /home/oss/aosp                 c            43444          2005850         14541914
    let mut total_files = 0;
    let mut total_blank_lines = 0;
    let mut total_nonblank_lines = 0;    
    for (dir_name, languages) in stats {
        let map = languages;
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
}
