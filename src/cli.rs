use std::path::PathBuf;

use clap::Parser;

use crate::Context;

pub const ANSII_GRAY: &str = "\x1b[90m";
pub const ANSII_RED: &str = "\x1b[31m";
pub const ANSII_GREEN: &str = "\x1b[32m";
pub const ANSII_YELLOW: &str = "\x1b[33m";
pub const ANSII_BLUE: &str = "\x1b[34m";
pub const ANSII_MAGENTA: &str = "\x1b[35m";
pub const ANSII_CYAN: &str = "\x1b[36m";
pub const ANSII_CLEAR: &str = "\x1b[0m";

#[derive(Parser)]
#[clap(name = "cargo-swoop")]
pub struct Args {
    pub search_dir: Option<PathBuf>,

    #[clap(long)]
    pub follow_symlinks: bool,

    #[clap(long)]
    pub show_empty: bool,
}

pub struct Stats {
    pub total_size: u64,
    pub non_empty_crates: usize,
}

pub fn crate_stats(ctx: &Context) -> Stats {
    let mut total_size = 0;
    let mut non_empty_crates = 0;
    for c in ctx.crates.iter() {
        if let Some(size) = c.target_dir_size {
            total_size += size;
            non_empty_crates += 1;
        }
    }
    Stats {
        total_size,
        non_empty_crates,
    }
}

pub fn display_crates(ctx: &Context, args: &Args, stats: &Stats) {
    for c in ctx.crates.iter() {
        if let Some(size) = c.target_dir_size {
            print!("{} ", Size(size));
        } else if args.show_empty {
            print!("  {ANSII_GRAY}<empty>{ANSII_CLEAR} ");
        } else {
            continue;
        }
        println!("{ANSII_BLUE}{}{ANSII_CLEAR}", c.path.display());
    }

    if stats.total_size > 0 {
        println!("{ANSII_GRAY}----------{ANSII_CLEAR}");
        println!("{} ", Size(stats.total_size));
    } else if stats.non_empty_crates == 0 && !args.show_empty {
        if ctx.crates.is_empty() {
            println!("{ANSII_GRAY}no crates found{ANSII_CLEAR}");
        } else {
            println!("{ANSII_GRAY}only empty crates found{ANSII_CLEAR}");
        }
    }
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

pub fn confirmation(prompt: &str) -> bool {
    let stdin = std::io::stdin();
    println!("{prompt} [y/N]");
    let mut buf = String::new();
    stdin.read_line(&mut buf).unwrap();

    let input = buf.trim();
    input.eq_ignore_ascii_case("y") || input.eq_ignore_ascii_case("yes")
}
