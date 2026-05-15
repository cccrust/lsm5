use std::time::Instant;

pub struct Timer {
    start: Instant,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            start: Instant::now(),
        }
    }

    pub fn elapsed_us(&self) -> u64 {
        self.start.elapsed().as_micros() as u64
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.start.elapsed().as_millis() as u64
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

impl Default for Timer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Clone)]
pub struct PerfStats {
    put_count: u64,
    put_total_us: u64,
    get_count: u64,
    get_total_us: u64,
    scan_count: u64,
    scan_total_us: u64,
    flush_count: u64,
    flush_total_us: u64,
    compact_count: u64,
    compact_total_us: u64,
}

impl PerfStats {
    pub fn record_put(&mut self, us: u64) {
        self.put_count += 1;
        self.put_total_us += us;
    }

    pub fn record_get(&mut self, us: u64) {
        self.get_count += 1;
        self.get_total_us += us;
    }

    pub fn record_scan(&mut self, us: u64) {
        self.scan_count += 1;
        self.scan_total_us += us;
    }

    pub fn record_flush(&mut self, us: u64) {
        self.flush_count += 1;
        self.flush_total_us += us;
    }

    pub fn record_compact(&mut self, us: u64) {
        self.compact_count += 1;
        self.compact_total_us += us;
    }

    pub fn avg_put_us(&self) -> u64 {
        if self.put_count == 0 {
            0
        } else {
            self.put_total_us / self.put_count
        }
    }

    pub fn avg_get_us(&self) -> u64 {
        if self.get_count == 0 {
            0
        } else {
            self.get_total_us / self.get_count
        }
    }

    pub fn avg_scan_us(&self) -> u64 {
        if self.scan_count == 0 {
            0
        } else {
            self.scan_total_us / self.scan_count
        }
    }

    pub fn avg_flush_ms(&self) -> u64 {
        if self.flush_count == 0 {
            0
        } else {
            self.flush_total_us / self.flush_count / 1000
        }
    }

    pub fn summary(&self) -> String {
        format!(
            "Performance Summary:\n  put:   avg {}μs, count {}\n  get:   avg {}μs, count {}\n  scan:  avg {}μs, count {}\n  flush: avg {}ms, count {}",
            self.avg_put_us(),
            self.put_count,
            self.avg_get_us(),
            self.get_count,
            self.avg_scan_us(),
            self.scan_count,
            self.avg_flush_ms(),
            self.flush_count
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer() {
        let mut timer = Timer::new();
        std::thread::sleep(std::time::Duration::from_micros(100));
        assert!(timer.elapsed_us() >= 100);
    }

    #[test]
    fn test_perf_stats() {
        let mut stats = PerfStats::default();
        stats.record_put(50);
        stats.record_put(100);
        assert_eq!(stats.avg_put_us(), 75);
    }
}
