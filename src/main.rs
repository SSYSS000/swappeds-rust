use std::{io::{Read, BufRead, BufReader}, ffi::OsStr};
use std::fs::{File, ReadDir, DirEntry};
use std::path::{Path, PathBuf};
use std::iter::Filter;

fn is_all_digits(s: &str) -> bool {
    s.chars().all(|x| x.is_digit(10))
}

#[derive(Default)]
struct ProcessStatus {
    pid: i32,
    process_name: String,
    vm_swap: Option<usize>
}

struct ProcessStatusReader {
    iter: Filter<ReadDir, fn(Result<DirEntry, std::io::Error>) -> bool> 
}


#[cfg(target_os = "linux")]
fn create_process_status_reader() -> std::io::Result<ProcessStatusReader> {
    Ok(ProcessStatusReader {
        iter: std::fs::read_dir("/proc")?.filter(|entry| {
            let path = entry.unwrap().path();

            path.is_dir() && match path.file_name().and_then(OsStr::to_str) {
                Some(dir_name) => is_all_digits(dir_name),
                None => false
            }
        })
    })
}

impl ProcessStatusReader {
}

#[derive(Debug)]
enum ReadError {
    IoError(std::io::Error),
    ParseError(std::num::ParseIntError)
}

impl From<std::io::Error> for ReadError {
    fn from(e: std::io::Error) -> Self {
        ReadError::IoError(e)
    }
}

impl From<std::num::ParseIntError> for ReadError {
    fn from(e: std::num::ParseIntError) -> Self {
        ReadError::ParseError(e)
    }
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
                status.vm_swap = Some(value.strip_suffix(" kB").unwrap().parse()?)
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
            Ok(p) => read_process_status(p.join("status")),
            Err(e) => Err(e.into())
        })
    }
}

fn main() {
    println!("     PID          SWAP     NAME");

    for status in create_process_status_reader().unwrap() {
        match status {
            Err(e) => eprintln!("{:?}", e),

            Ok(status) => {
                println!("{: >8}     {: >6} kB     {}",
                    status.pid,
                    status.vm_swap.unwrap_or(0),
                    status.process_name
                );
            }
        }
    }
}
