use std::{io, fs};
use std::path::Path;
use std::collections::{HashMap};
use std::collections::hash_map::Entry;
use std::sync::{Arc, Mutex};
use async_recursion::async_recursion;
use tokio::task::{self, JoinHandle};
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

#[async_recursion]
pub async fn count_files(dir: Box<Path>, types: Arc<Box<Vec<String>>>, map: Arc<Mutex<HashMap<String, LinesStat>>>, dir_parallel: Arc<Box<bool>>) -> Result<(u32, u32), io::Error> {
    let mut total_files = 0;
    let mut matched_files = 0;
    let mut handles: Vec<JoinHandle<_>> = Vec::new();
    let mut handles2: Vec<JoinHandle<_>> = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let c_types = Arc::clone(&types);
            let c_map = Arc::clone(&map);
            let c_dir_parallel = Arc::clone(&dir_parallel);
            handles.push(tokio::spawn(task::block_in_place(|| {
                count_files(path.into(), c_types, c_map, c_dir_parallel)
            })));
        } else {
            let ext = path.extension();
            total_files += 1;
            match ext{
                Some(extval) => {
                    let extname = String::from(extval.to_str().unwrap());
                    let c_types = Arc::clone(&types);
                    let found_types = c_types.iter().find(|&x| x == extname.as_str());
                    match  found_types {
                        Some(_) => {
                            matched_files += 1;
                            let c_map = Arc::clone(&map);
                            let c_dir_parallel = Arc::clone(&dir_parallel);
                            if **c_dir_parallel {
                                stat_lines_sync(path.into(), c_map, extname);
                            } else {
                                handles2.push(tokio::spawn(task::block_in_place(|| {
                                    stat_lines(path.into(), c_map, extname)
                                })));                            
                            }
                        },
                        _ => {},
                    }
                }
                _ => {},
            }
        }
    }
    for h in handles2 {
        match h.await {
            Err(err) => {
                println!("count_files(): Error in stat_lines().await result: {}", err);
            },
            _ => {},
        }
    }
    for h in handles {
        match h.await {
            Ok(h_result) => {
                let (sub_total_files, sub_matched_files)=h_result.unwrap();
                total_files += sub_total_files;
                matched_files += sub_matched_files;
            },
            Err(err) => {
                println!("count_files(): Error in await result: {}", err);
            },
        }
    }
    Ok((total_files, matched_files))
}

#[async_recursion]
pub async fn stat_lines(file_path: Box<Path>, map: Arc<Mutex<HashMap<String, LinesStat>>>, extname: String) {
    //let lines_stat = stat_lines(path.to_str().unwrap()).unwrap();
    let lines_stat: LinesStat;
    match stat_lines_opt(file_path) {
        Ok(lines_stat_) => {
            lines_stat = lines_stat_;
        },
        Err(err) => {
            println!("stat_lines(): Error occurred during process file: {}", err);
            return;
        },
    }
    let lang_name: String = String::from(extname.as_str());
    let l_map = &mut map.lock().unwrap();
    match l_map.entry(extname) {
        Entry::Occupied(mut map_entry) => {
            map_entry.get_mut().files_count += lines_stat.files_count;
            map_entry.get_mut().blank_lines_count += lines_stat.blank_lines_count;
            map_entry.get_mut().nonblank_lines_count += lines_stat.nonblank_lines_count;
        },
        Entry::Vacant(_) => {
            let mut new_linestat = LinesStat::new();
            new_linestat.files_count = lines_stat.files_count;
            new_linestat.blank_lines_count = lines_stat.blank_lines_count;
            new_linestat.nonblank_lines_count = lines_stat.nonblank_lines_count;
            l_map.insert(
                lang_name, 
                new_linestat,
            );
        },
    }
}

fn stat_lines_opt(file_path: Box<Path>) -> Result<LinesStat, io::Error>{
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


pub fn count_files_sync(dir: Box<Path>, types: Arc<Box<Vec<String>>>, map: Arc<Mutex<HashMap<String, LinesStat>>>) -> Result<(u32, u32), io::Error> {
    let mut total_files = 0;
    let mut matched_files = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            let c_types = Arc::clone(&types);
            let c_map = Arc::clone(&map);
            let (sub_total_files, sub_matched_files) = count_files_sync(path.into(), c_types, c_map).unwrap();
            total_files += sub_total_files;
            matched_files += sub_matched_files;
        } else {
            let ext = path.extension();
            total_files += 1;
            match ext{
                Some(extval) => {
                    let extname = String::from(extval.to_str().unwrap());
                    let c_types = Arc::clone(&types);
                    let found_types = c_types.iter().find(|&x| x == extname.as_str());
                    match  found_types {
                        Some(_) => {
                            matched_files += 1;
                            let c_map = Arc::clone(&map);
                            stat_lines_sync(path.into(), c_map, extname);
                        },
                        _ => {},
                    }
                }
                _ => {},
            }
        }
    }
    Ok((total_files, matched_files))
}

pub fn stat_lines_sync(file_path: Box<Path>, map: Arc<Mutex<HashMap<String, LinesStat>>>, extname: String) {
    //let lines_stat = stat_lines(path.to_str().unwrap()).unwrap();
    let lines_stat = stat_lines_opt(file_path).unwrap();
    let lang_name: String = String::from(extname.as_str());
    let l_map = &mut map.lock().unwrap();
    match l_map.entry(extname) {
        Entry::Occupied(mut map_entry) => {
            map_entry.get_mut().files_count += lines_stat.files_count;
            map_entry.get_mut().blank_lines_count += lines_stat.blank_lines_count;
            map_entry.get_mut().nonblank_lines_count += lines_stat.nonblank_lines_count;
        },
        Entry::Vacant(_) => {
            let mut new_linestat = LinesStat::new();
            new_linestat.files_count = lines_stat.files_count;
            new_linestat.blank_lines_count = lines_stat.blank_lines_count;
            new_linestat.nonblank_lines_count = lines_stat.nonblank_lines_count;
            l_map.insert(
                lang_name, 
                new_linestat,
            );
        },
    }
}


// fn stat_lines_normal(file_path: Box<Path>) -> Result<LinesStat, io::Error>{
//     //println!("calling stat_lines_normal() ...");
//     let mut files_count = 0;
//     let mut blank_lines_count = 0;
//     let mut nonblank_lines_count = 0;
//     let contents = fs::read_to_string(file_path);
//     match contents {
//         Ok(contents) => {
//             let lines: Vec<&str> = contents.split('\n').collect();
//             for line in lines {
//                 if line.trim() == "" {
//                     blank_lines_count += 1;
//                 } else {
//                     nonblank_lines_count += 1;
//                 }        
//             }
//             files_count = 1;
//         },
//         Err(_err) => {

//         },
//     }
//     //let contents = String::from_utf8_unchecked(rawdata);
//     Ok(LinesStat {files_count, blank_lines_count, nonblank_lines_count})
// }


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
