extern crate chrono;
extern crate libc;
extern crate clipboard;
extern crate shlex;

use std::path::{Path,PathBuf};
use std::process::{Command,Output,exit};
use std::env;
use std::fs;
use std::io::{self};

use chrono::prelude::*;

#[derive(Clone)]
struct TempFile {
    path: PathBuf,
}

struct Shell {
    path: PathBuf,
    argv: Vec<String>,
    cmd_idx: usize,
}

fn main() {

    if !in_path("xclip") {
        println!("xclip not found in path, please install");
        exit(1);
    }

    let file = TempFile::new("vim-anwhere")
        .expect("Could not create temporary file");

    let shell = get_shell().unwrap_or_else(|| {
        println!("Failed to parse shell from environment");
        println!("\tSet VIM_ANYWHERE_TERM to your preferred terminal");
        println!("\tUse %s in place of the vim command to execute");
        println!("\tEx. 'terminator -e \"%s\"'");
        exit(1);
    });

    shell.spawn_cmd(
        &format!(
            "vim +star {}",
            file.path.to_str().unwrap()
        )
    ).unwrap_or_else(|_| {
        println!("Failed to spawn shell");
        println!("\tSet VIM_ANYWHERE_TERM to your preferred terminal");
        println!("\tUse %s in place of the vim command to execute");
        println!("\tEx. 'terminator -e \"%s\"'");
        exit(1);
    });

    file.copy();
}

fn in_path(prog: &str) -> bool {
    if let Ok(path) = env::var("PATH") {
        path.split(":").any(|x| {
            Path::new(x).join(prog).exists()
        })
    } else {
        false
    }
}

fn get_shell() -> Option<Shell> {
    match env::var("VIM_ANYWHERE_TERM") {
        Ok(val) => match Shell::parse(&val) {
            Some(val) => Some(val),
            None => {
                None
            }
        },
        Err(_) => Shell::parse("xterm -e \"%s\""),
    }
}

impl TempFile {
    fn new(dir: &str) -> Result<Self,io::Error> {
        let tmp_root = env::temp_dir();
        let tmp_dir = tmp_root.join(Path::new(dir));

        if !tmp_dir.exists() {
            fs::create_dir(tmp_dir.clone())
                .expect(
                    &format!(
                        "Could not create temporary directory: {}",
                        tmp_dir.to_str().unwrap())
                );
        }

        let local: DateTime<Local> = Local::now();
        let local_fmt = local.format("doc-%y%m%d%H%M%S").to_string();
        let path = tmp_dir.join(&local_fmt);

        if path.exists() {
            fs::remove_file(path.clone())?;
        }

        Ok(TempFile {
            path: path,
        })
    }

    fn copy(self)  {
        // quick and dirty fix
        // rust-clipboard doesn't persist after application exit
        let pid = unsafe { libc::fork() };

        if pid > 0 {
            return;
        }

        unsafe { libc::close(libc::STDIN_FILENO) };
        unsafe { libc::close(libc::STDOUT_FILENO) };
        unsafe { libc::close(libc::STDERR_FILENO) };

        Command::new("xclip")
            .args(&["-r", "-selection","c",self.path.to_str().unwrap()])
            .output()
            .expect("failed to launch xclip");
    }
}

impl Shell {
    fn spawn_cmd(self, cmd: &str) -> io::Result<Output> {
        let mut argv = self.argv.clone();
        argv.insert(self.cmd_idx, cmd.to_string());

        Command::new(self.path.clone())
            .args(&argv)
            .output()
    }

    fn parse(fmt: &str) -> Option<Self> {
        let mut args = match shlex::split(fmt) {
            Some(x) => x,
            None => { return None; }
        };

        let path = PathBuf::from(args.remove(0));

        let idx = match args.iter().position(|ref x| &**x == "%s") {
            Some(x) => x,
            None => { return None; }
        };

        args.remove(idx);

        Some(Shell {
            path: path,
            argv: args,
            cmd_idx: idx,
        })
    }
}
