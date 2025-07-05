use std::fs::ReadDir;
use std::path::PathBuf;

pub struct DirIter<T> {
    follow_symlinks: bool,
    stack: Vec<T>,
}

impl<T: DirStackEntry> DirIter<T> {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        let mut iter = Self {
            stack: Vec::new(),
            follow_symlinks: false,
        };
        iter.enter_dir(path)?;
        Ok(iter)
    }

    pub fn follow_symlinks(mut self, follow_symlinks: bool) -> Self {
        self.follow_symlinks = follow_symlinks;
        self
    }

    /// Panics when called after the iterator is finished.
    pub fn current_dir(&mut self) -> &mut T {
        self.stack.last_mut().expect("stack to be non-empty")
    }
}

pub trait DirStackEntry {
    fn new(path: PathBuf, iter: ReadDir) -> Self;

    fn iter(&mut self) -> &mut ReadDir;
}

pub enum DirIterItem<T> {
    /// A file was found.
    File(std::fs::DirEntry),
    /// A directory was found.
    Dir(PathBuf),
    /// The directory has been completely traversed.
    FinishedDir(T),
}

impl<T: DirStackEntry> DirIter<T> {
    pub fn next(&mut self) -> std::io::Result<Option<DirIterItem<T>>> {
        let Some(dir) = self.stack.last_mut() else {
            return Ok(None);
        };

        let item = loop {
            let Some(entry) = dir.iter().next() else {
                let dir = self.stack.pop().unwrap();
                break DirIterItem::FinishedDir(dir);
            };
            let entry = entry?;

            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                break DirIterItem::Dir(entry.path());
            } else if file_type.is_file() {
                break DirIterItem::File(entry);
            } else if file_type.is_symlink() && self.follow_symlinks {
                todo!("folow symlink")
            }
        };

        Ok(Some(item))
    }

    pub fn enter_dir(&mut self, path: PathBuf) -> std::io::Result<()> {
        let iter = std::fs::read_dir(&path)?;
        let dir = T::new(path, iter);
        self.stack.push(dir);
        Ok(())
    }
}
