use crate::{Args, Context};

const ANSII_GRAY: &str = "\x1b[90m";
const ANSII_RED: &str = "\x1b[31m";
const ANSII_GREEN: &str = "\x1b[32m";
const ANSII_YELLOW: &str = "\x1b[33m";
const ANSII_BLUE: &str = "\x1b[34m";
const ANSII_MAGENTA: &str = "\x1b[35m";
const ANSII_CYAN: &str = "\x1b[36m";
const ANSII_CLEAR: &str = "\x1b[0m";

pub fn crates(ctx: &Context, args: &Args) {
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
