//use std::error::Error;

#[derive(Debug)]
pub enum RunMode {
    SingleThread,
    MultipleThread,
    MultipleThread4Dir,
}

#[derive(Debug)]
pub struct Config{
    pub run_mode: RunMode,
    pub mem_zero_copy: bool,
    pub dirs: Vec<String>,
    pub types: Vec<String>,
}

impl Config {
    pub fn new(mut args: std::env::Args) -> Result<Config, &'static str>{
        args.next();  //skit the command itself.
        let mut run_mode = RunMode::SingleThread;
        let mut mem_zero_copy = false;
        let mut dirs: Vec<String> = vec![];
        let mut types: Vec<String> = vec![];
        let mut state = "IDLE";
        loop {
            match args.next() {
                Some(arg) => {
                    if arg == "-t" {
                        state = "WAIT_TYPE";
                    } else if arg == "-f" {
                        run_mode = RunMode::MultipleThread;
                        state = "IDLE";
                    } else if arg == "-d" {
                        run_mode = RunMode::MultipleThread4Dir;
                        state = "IDLE";
                    } else if arg == "-z" {
                        mem_zero_copy = true;
                        state = "IDLE";
                    } else {
                        if state == "IDLE" {
                            dirs.push(arg);
                        } else if state == "WAIT_TYPE" {
                            types.push(arg);
                        }
                    }
                },
                None => break,
            };
        }

        Ok(Config{run_mode, mem_zero_copy, dirs, types})
    }    
}
