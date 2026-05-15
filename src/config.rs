use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub dir: PathBuf,
    pub memtable_size_limit: usize,
    pub max_sstable_size: u64,
    pub sync_writes: bool,
}

impl Config {
    pub fn new(dir: impl AsRef<std::path::Path>) -> Self {
        Config {
            dir: dir.as_ref().to_path_buf(),
            memtable_size_limit: 4 * 1024 * 1024,
            max_sstable_size: 64 * 1024 * 1024,
            sync_writes: false,
        }
    }

    pub fn memtable_size_limit(mut self, bytes: usize) -> Self {
        self.memtable_size_limit = bytes;
        self
    }

    pub fn max_sstable_size(mut self, bytes: u64) -> Self {
        self.max_sstable_size = bytes;
        self
    }

    pub fn sync_writes(mut self, sync: bool) -> Self {
        self.sync_writes = sync;
        self
    }
}
