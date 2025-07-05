use std::fs::ReadDir;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::iter::{DirIter, DirIterItem, DirStackEntry};

mod iter;

const ANSII_GRAY: &str = "\x1b[90m";
const ANSII_RED: &str = "\x1b[31m";
const ANSII_GREEN: &str = "\x1b[32m";
const ANSII_YELLOW: &str = "\x1b[33m";
const ANSII_BLUE: &str = "\x1b[34m";
const ANSII_MAGENTA: &str = "\x1b[35m";
const ANSII_CYAN: &str = "\x1b[36m";
const ANSII_CLEAR: &str = "\x1b[0m";

#[derive(Parser)]
#[clap(name = "cargo-swoop")]
pub struct Args {
    search_dir: Option<PathBuf>,

    #[clap(long)]
    follow_symlinks: bool,

    #[clap(long)]
    show_empty: bool,
}

pub struct Context {
    crates: Vec<CrateInfo>,
}

impl Context {
    pub fn new() -> Self {
        Self { crates: Vec::new() }
    }
}

struct CrateInfo {
    path: PathBuf,
    target_dir_size: Option<u64>,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    let mut args: Vec<String> = std::env::args().collect();
    // remove "swoop" when invoked `cargo swoop`
    // https://github.com/rust-lang/cargo/issues/7653
    if args.len() > 1 && args[1] == "swoop" {
        args.remove(1);
    }
    let args = Args::parse_from(args);

    let dir_path = match args.search_dir.clone() {
        Some(dir) => dir,
        None => std::env::current_dir()?,
    };

    let mut ctx = Context::new();
    find_crates(&mut ctx, &args, dir_path)?;

    ctx.crates.sort_by_key(|c| c.target_dir_size);

    let mut total_size = 0;
    let mut only_empty_crates = true;
    for c in ctx.crates.iter() {
        if let Some(size) = c.target_dir_size {
            print!("{} ", Size(size));
            total_size += size;
            only_empty_crates = false;
        } else if args.show_empty {
            print!("  {ANSII_GRAY}<empty>{ANSII_CLEAR} ");
        } else {
            continue;
        }
        println!("{ANSII_BLUE}{}{ANSII_CLEAR}", c.path.display());
    }

    if total_size > 0 {
        println!("{ANSII_GRAY}----------{ANSII_CLEAR}");
        println!("{} ", Size(total_size));
    } else if only_empty_crates && !args.show_empty {
        if ctx.crates.is_empty() {
            println!("{ANSII_GRAY}no crates found{ANSII_CLEAR}");
        } else {
            println!("{ANSII_GRAY}only empty crates found{ANSII_CLEAR}");
        }
    }

    Ok(())
}

struct Size(u64);

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Size(size) = *self;

        const KB: u64 = 1024;
        const MB: u64 = 1024 * KB;
        const GB: u64 = 1024 * MB;
        const TB: u64 = 1024 * GB;

        #[rustfmt::skip]
        match self.0 {
            0..KB => write!(f, "{size:6} {ANSII_GREEN}B {ANSII_CLEAR}")?,
            0..MB => write!(f, "{:6.2} {ANSII_CYAN   }KB{ANSII_CLEAR}", size as f64 / KB as f64)?,
            0..GB => write!(f, "{:6.2} {ANSII_YELLOW }MB{ANSII_CLEAR}", size as f64 / MB as f64)?,
            0..TB => write!(f, "{:6.2} {ANSII_MAGENTA}GB{ANSII_CLEAR}", size as f64 / GB as f64)?,
            TB.. =>  write!(f, "{:6.2} {ANSII_RED    }TB{ANSII_CLEAR}", size as f64 / TB as f64)?,
        };

        Ok(())
    }
}

struct DirContext {
    path: PathBuf,
    iter: ReadDir,
    has_manifest: bool,
    target_dir: Option<PathBuf>,
    /// Whether this dir has been added to the list of crates.
    done: bool,
}

impl DirStackEntry for DirContext {
    fn new(path: PathBuf, iter: ReadDir) -> Self {
        Self {
            path,
            iter,
            has_manifest: false,
            target_dir: None,
            done: false,
        }
    }

    fn iter(&mut self) -> &mut ReadDir {
        &mut self.iter
    }
}

fn find_crates(ctx: &mut Context, args: &Args, path: PathBuf) -> anyhow::Result<()> {
    let mut dir_iter = DirIter::<DirContext>::new(path)?.follow_symlinks(args.follow_symlinks);

    while let Some(item) = dir_iter.next()? {
        match item {
            DirIterItem::File(entry) => {
                let path = entry.path();
                let Some(file_name) = path.file_name() else {
                    continue;
                };
                if file_name == "Cargo.toml" {
                    let dir = dir_iter.current_dir();
                    dir.has_manifest = true;
                    try_compute_target_size(ctx, args, dir)?;
                }
            }
            DirIterItem::Dir(path) => {
                if path.file_name().is_some_and(|name| name == "target") {
                    let dir = dir_iter.current_dir();
                    dir.target_dir = Some(path);
                    try_compute_target_size(ctx, args, dir)?;
                    continue;
                }

                // Don't enter hidden directories.
                if !path.starts_with(".") {
                    dir_iter.enter_dir(path)?;
                }
            }
            DirIterItem::FinishedDir(dir) => {
                if !dir.done && dir.has_manifest {
                    ctx.crates.push(CrateInfo {
                        path: dir.path,
                        target_dir_size: None,
                    });
                }
            }
        }
    }

    Ok(())
}

struct DirSizeContext {
    iter: ReadDir,
}

impl DirStackEntry for DirSizeContext {
    fn new(_: PathBuf, iter: ReadDir) -> Self {
        Self { iter }
    }

    fn iter(&mut self) -> &mut ReadDir {
        &mut self.iter
    }
}

fn try_compute_target_size(
    ctx: &mut Context,
    args: &Args,
    dir: &mut DirContext,
) -> anyhow::Result<()> {
    if !dir.has_manifest {
        return Ok(());
    }
    let Some(path) = dir.target_dir.clone() else {
        return Ok(());
    };

    dir.done = true;

    let mut size = 0;

    let mut dir_iter =
        DirIter::<DirSizeContext>::new(path.clone())?.follow_symlinks(args.follow_symlinks);
    while let Some(item) = dir_iter.next()? {
        match item {
            DirIterItem::File(entry) => {
                size += entry.metadata()?.len();
            }
            DirIterItem::Dir(path) => dir_iter.enter_dir(path)?,
            DirIterItem::FinishedDir(_) => (),
        }
    }

    ctx.crates.push(CrateInfo {
        path,
        target_dir_size: Some(size),
    });

    Ok(())
}
