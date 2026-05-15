use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    pub dir: PathBuf,
    pub memtable_size_limit: usize,
    pub max_sstable_size: u64,
    pub sync_writes: bool,
    pub l0_compaction_trigger: usize,
    pub level_size_multiplier: u64,
}

impl Config {
    pub fn new(dir: impl AsRef<std::path::Path>) -> Self {
        Config {
            dir: dir.as_ref().to_path_buf(),
            memtable_size_limit: 4 * 1024 * 1024,
            max_sstable_size: 64 * 1024 * 1024,
            sync_writes: false,
            l0_compaction_trigger: 4,
            level_size_multiplier: 10,
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

    pub fn l0_compaction_trigger(mut self, count: usize) -> Self {
        self.l0_compaction_trigger = count;
        self
    }

    pub fn level_size_multiplier(mut self, multiplier: u64) -> Self {
        self.level_size_multiplier = multiplier;
        self
    }
}
