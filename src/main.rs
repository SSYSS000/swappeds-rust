extern crate getopts;

use std::{
    io::{self, BufRead, BufReader},
    ffi::OsStr,
    fs::{File, ReadDir, DirEntry},
    path::Path,
    iter::Filter,
    env
};

fn is_all_digits(s: &str) -> bool {
    s.chars().all(|x| x.is_digit(10))
}

#[derive(Debug)]
enum ReadError {
    IoError(io::Error),
    InvalidValue
}

impl From<io::Error> for ReadError {
    fn from(e: io::Error) -> Self {
        ReadError::IoError(e)
    }
}

impl From<std::num::ParseIntError> for ReadError {
    fn from(_: std::num::ParseIntError) -> Self {
        ReadError::InvalidValue
    }
}

#[derive(Default)]
struct ProcessStatus {
    pid: i32,
    process_name: String,
    vm_swap: Option<usize>
}

struct ProcessStatusReader {
    iter: Filter<ReadDir, fn(&io::Result<DirEntry>) -> bool> 
}

fn create_process_status_reader() -> io::Result<ProcessStatusReader> {
    fn is_process_subdir(entry: &io::Result<DirEntry>) -> bool {
        let path = entry.as_ref().unwrap().path();

        path.is_dir() && match path.file_name().and_then(OsStr::to_str) {
            Some(dir_name) => is_all_digits(dir_name),
            None => false
        }
    }

    Ok(ProcessStatusReader {
        iter: std::fs::read_dir("/proc")?.filter(is_process_subdir)
    })
}

fn read_process_status<P: AsRef<Path>>(path: P) -> Result<ProcessStatus, ReadError> {
    let f = File::open(path)?;
    let mut reader = BufReader::new(f);
    let mut line = String::new();
    let mut status = ProcessStatus::default();

    while reader.read_line(&mut line)? != 0 {
        let (field, value) = match line.split_once(':') {
            None         => continue,
            Some((k, v)) => (k, v.trim())
        };

        match field {
            "Pid"       => status.pid = value.parse()?,
            "Name"      => status.process_name.push_str(value),
            "VmSwap"    => {
                status.vm_swap = Some(
                    value.strip_suffix(" kB")
                    .ok_or(ReadError::InvalidValue)?
                    .parse()?
                )
            },
            _ => ()
        }

        line.clear();
    }

    Ok(status)
}

impl Iterator for ProcessStatusReader {
    type Item = Result<ProcessStatus, ReadError>;

    fn next(&mut self) -> Option<Self::Item> {
        Some(match self.iter.next()? {
            Ok(v)  => read_process_status(v.path().join("status")),
            Err(e) => Err(e.into())
        })
    }
}

fn main() {
    static OPT_TOTAL: &str = "c";
    static OPT_HELP: &str  = "h";

    let mut total: usize = 0;

    let mut opts = getopts::Options::new();
    opts.optflag(OPT_TOTAL, "", "produce total swap usage");
    opts.optflag(OPT_HELP, "help", "print this help text");

    let matches = opts.parse(env::args()).unwrap();

    if matches.opt_present(OPT_HELP) {
        println!("usage: {} [options]\n{}", "pswap", opts.usage(""));
        return;
    }

    println!("     PID          SWAP     NAME");

    for status in create_process_status_reader().unwrap() {
        match status {
            Err(e) => eprintln!("{:?}", e),

            Ok(status) => {
                let swap_kb = status.vm_swap.unwrap_or(0);
                total += swap_kb;

                println!("{: >8}     {: >6} kB     {}",
                    status.pid,
                    swap_kb,
                    status.process_name
                );
            }
        }
    }

    if matches.opt_present(OPT_TOTAL) {
        println!("Total     {} kB", total);
    }
}
