use std::fs::ReadDir;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;

use crate::cli::{ANSII_BLUE, ANSII_CLEAR, ANSII_GRAY, ANSII_RED, Args};
use crate::iter::{DirIter, DirIterItem, DirStackEntry};

mod cli;
mod iter;

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

    let stats = cli::crate_stats(&ctx);
    cli::display_crates(&ctx, &args, &stats);

    if stats.non_empty_crates > 0 {
        let remove_dirs = cli::confirmation("\nremove target directories");
        if remove_dirs {
            for c in ctx.crates.iter() {
                if c.target_dir_size.is_some() {
                    let target_dir = c.path.join("target");
                    println!(
                        "{ANSII_RED}removing{ANSII_CLEAR} {ANSII_BLUE}{}{ANSII_CLEAR}",
                        target_dir.display()
                    );
                    std::fs::remove_dir_all(target_dir)?;
                }
            }
        } else {
            println!("{ANSII_GRAY}cancelled{ANSII_CLEAR}");
        }
    }

    Ok(())
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
        path: dir.path.clone(),
        target_dir_size: Some(size),
    });

    Ok(())
}
