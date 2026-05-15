use std::fmt;

#[derive(Debug, Clone, Default)]
pub struct DbStats {
    pub memtable_entries: usize,
    pub memtable_size_bytes: usize,
    pub level_counts: Vec<usize>,
    pub level_sizes: Vec<u64>,
    pub reads: u64,
    pub writes: u64,
    pub flushes: u64,
    pub compactions: u64,
}

impl DbStats {
    pub fn new(
        memtable_entries: usize,
        memtable_size_bytes: usize,
        level_counts: Vec<usize>,
        level_sizes: Vec<u64>,
    ) -> Self {
        Self {
            memtable_entries,
            memtable_size_bytes,
            level_counts,
            level_sizes,
            ..Default::default()
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn with_metrics(
        memtable_entries: usize,
        memtable_size_bytes: usize,
        level_counts: Vec<usize>,
        level_sizes: Vec<u64>,
        reads: u64,
        writes: u64,
        flushes: u64,
        compactions: u64,
    ) -> Self {
        Self {
            memtable_entries,
            memtable_size_bytes,
            level_counts,
            level_sizes,
            reads,
            writes,
            flushes,
            compactions,
        }
    }

    pub fn total_sstables(&self) -> usize {
        self.level_counts.iter().sum()
    }

    pub fn total_size_bytes(&self) -> u64 {
        self.level_sizes.iter().sum()
    }

    pub fn active_level_count(&self) -> usize {
        self.level_counts.iter().filter(|&&c| c > 0).count()
    }

    pub fn max_level_size(&self) -> u64 {
        *self.level_sizes.iter().max().unwrap_or(&0)
    }

    pub fn avg_sstable_size(&self) -> f64 {
        let total = self.total_sstables();
        if total == 0 {
            return 0.0;
        }
        self.total_size_bytes() as f64 / total as f64
    }

    pub fn is_compaction_needed(&self, level: usize, threshold: u64) -> bool {
        self.level_sizes.get(level).is_some_and(|&s| s > threshold)
    }

    pub fn increment_reads(&mut self) {
        self.reads += 1;
    }

    pub fn increment_writes(&mut self) {
        self.writes += 1;
    }

    pub fn increment_flushes(&mut self) {
        self.flushes += 1;
    }

    pub fn increment_compactions(&mut self) {
        self.compactions += 1;
    }
}

impl fmt::Display for DbStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "=== LSM5 Database Statistics ===")?;
        writeln!(
            f,
            "MemTable: {} entries, {} bytes",
            self.memtable_entries, self.memtable_size_bytes
        )?;
        for (i, (count, size)) in self.level_counts.iter().zip(&self.level_sizes).enumerate() {
            if *count > 0 {
                writeln!(f, "  L{}: {} SSTables, {} bytes", i, count, size)?;
            }
        }
        writeln!(f)?;
        writeln!(f, "Operations:")?;
        writeln!(f, "  Reads:     {}", self.reads)?;
        writeln!(f, "  Writes:    {}", self.writes)?;
        writeln!(f, "  Flushes:   {}", self.flushes)?;
        writeln!(f, "  Compactions: {}", self.compactions)?;
        Ok(())
    }
}
