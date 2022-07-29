use std::{io, fs};
use std::fs::DirEntry;
use std::path::Path;
use std::collections::{HashMap};
use std::collections::hash_map::Entry;
use std::sync::{Arc};
use rayon::prelude::*;
// use crate::log::log;


#[derive(Debug,Default)]
pub struct LinesStat {
    pub files_count: u32,
    pub blank_lines_count: u32,
    pub nonblank_lines_count: u32,
}

impl LinesStat {
    pub fn new() -> LinesStat {
        LinesStat::default()
    }
}

pub fn scan_dir(dir: &Path, types: Arc<Box<Vec<String>>>) -> HashMap<String, LinesStat> {
    let readir = fs::read_dir(dir);
    match readir {
        Ok(entries_) => {
            let mut entries: std::vec::Vec<DirEntry> = vec![];
            for entry in entries_ {
                match entry {
                    Ok(entry_) => {
                        entries.push(entry_);
                    },
                    Err(_) => {
                        println!("scan_dir(): invalid entry")
                    },
                }
            }
            entries.par_iter()
                .map(|entry| {
                    let c_types = Arc::clone(&types);
                    stat_lines(entry, c_types)
                })
                .reduce(|| HashMap::new(), |a, b| collect_lines(a, b))
        },
        Err(_) => {
            HashMap::new()
        },
    }
}

pub fn collect_lines(mut a: HashMap<String, LinesStat>, b: HashMap<String, LinesStat>)  -> HashMap<String, LinesStat> {
    for (lang, lines) in b.iter(){
        let lang_: String = String::from(lang.as_str());
        match a.entry(lang_) {
            Entry::Occupied(mut map_entry) => {
                map_entry.get_mut().files_count += lines.files_count;
                map_entry.get_mut().blank_lines_count += lines.blank_lines_count;
                map_entry.get_mut().nonblank_lines_count += lines.nonblank_lines_count;
            },
            Entry::Vacant(_) => {
                let mut new_linestat = LinesStat::new();
                let lang_name: String = String::from(lang.as_str());
                new_linestat.files_count = lines.files_count;
                new_linestat.blank_lines_count = lines.blank_lines_count;
                new_linestat.nonblank_lines_count = lines.nonblank_lines_count;
                a.insert(
                    lang_name, 
                    new_linestat,
                );
            },
        }
    }
    a
}

pub fn stat_lines(entry: &DirEntry, types: Arc<Box<Vec<String>>>) -> HashMap<String, LinesStat> {
    let path = entry.path();
    let file_type_ = entry.file_type();
    match file_type_ {
        Ok(_) => {
        },
        Err(_) => {
            return HashMap::new();
        },
    }
    let file_type = file_type_.unwrap();
    if file_type.is_symlink() {
        return HashMap::new();
    }
    if file_type.is_dir() {
        scan_dir(&path, types)
    } else {
        let ext = path.extension();
        match ext{
            Some(extval) => {
                let extname = String::from(extval.to_str().unwrap());
                let found_types = types.iter().find(|&x| x == extname.as_str());
                match  found_types {
                    Some(_) => {
                        return stat_file_lines(&path, extname)
                    },
                    _ => {},
                }
            }
            _ => {},
        }
        return HashMap::new();
    }
}


pub fn stat_file_lines(file_path: &Path, extname: String) -> HashMap<String, LinesStat> {
    let mut result = HashMap::new();
    match stat_lines_opt(file_path) {
        Ok(lines_stat) => {
            result.insert(extname, lines_stat);
        },
        Err(err) => {
            println!("stat_file_lines(): Error occurred during process file: {}", err);
        },
    }
    return result;
}



fn stat_lines_opt(file_path: &Path) -> Result<LinesStat, io::Error>{
    //println!("calling stat_lines_opt() ...");
    let mut blank_lines_count = 0;
    let mut nonblank_lines_count = 0;

    let rawdata = fs::read(file_path)?;
    let mut valid_ch_count = 0;
    let size = rawdata.len();
    for rbyte in rawdata {
        if rbyte == b'\n' {
            if valid_ch_count > 0 {
                nonblank_lines_count += 1;
            } else {
                blank_lines_count += 1;
            }
            valid_ch_count = 0;
            continue;
        } else if rbyte > 0x20 && rbyte <= 0x7e {
            valid_ch_count +=1;
        } else if rbyte == b'\t' || rbyte == b'\r' || rbyte == b' ' ||  rbyte == 0x0b || rbyte == 0x0c {
            continue
        } else {
            valid_ch_count +=1;
        }
    }
    if valid_ch_count > 0 {
        nonblank_lines_count += 1;
    } else if size > 0 {
        blank_lines_count += 1;
    }
    Ok(LinesStat {files_count: 1, blank_lines_count, nonblank_lines_count})
}


#[cfg(test)]

#[test]
fn test_stat_lines_1() {
    let file_path = "./Cargo.toml";
    let file_info = stat_lines(file_path).unwrap();
    assert_eq!(file_info.blank_lines_count,2);
}

#[test]
fn test_stat_lines_2() {
    let file_path = "./Cargo.toml";
    let file_info = stat_lines(file_path).unwrap();
    assert_eq!(file_info.nonblank_lines_count,7);
}
