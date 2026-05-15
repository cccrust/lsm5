use std::fmt;

#[derive(Clone, Debug)]
pub struct ScanPlan {
    pub range_start: Vec<u8>,
    pub range_end: Vec<u8>,
    pub estimated_keys: usize,
    pub levels_to_scan: Vec<usize>,
    pub use_bloom_filter: bool,
    pub use_index: bool,
    pub cost: CostLevel,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CostLevel {
    Low,
    Medium,
    High,
}

impl fmt::Display for ScanPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Scan Plan:")?;
        writeln!(
            f,
            "  Range: [{}, {}]",
            String::from_utf8_lossy(&self.range_start),
            String::from_utf8_lossy(&self.range_end)
        )?;
        writeln!(f, "  Estimated keys: {}", self.estimated_keys)?;
        writeln!(f, "  Levels to scan: {:?}", self.levels_to_scan)?;
        writeln!(f, "  Bloom filter: {}", self.use_bloom_filter)?;
        writeln!(f, "  Index: {}", self.use_index)?;
        writeln!(f, "  Cost: {:?}", self.cost)?;
        Ok(())
    }
}

impl ScanPlan {
    pub fn new(start: &[u8], end: &[u8]) -> Self {
        ScanPlan {
            range_start: start.to_vec(),
            range_end: end.to_vec(),
            estimated_keys: 0,
            levels_to_scan: Vec::new(),
            use_bloom_filter: true,
            use_index: true,
            cost: CostLevel::Medium,
        }
    }

    pub fn with_levels(mut self, levels: Vec<usize>) -> Self {
        self.levels_to_scan = levels;
        self
    }

    pub fn with_estimated_keys(mut self, keys: usize) -> Self {
        self.estimated_keys = keys;
        self
    }

    pub fn with_cost(mut self, cost: CostLevel) -> Self {
        self.cost = cost;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_plan_display() {
        let plan = ScanPlan::new(b"a", b"z")
            .with_levels(vec![0, 1])
            .with_estimated_keys(1000)
            .with_cost(CostLevel::Medium);

        let output = format!("{}", plan);
        assert!(output.contains("a"));
        assert!(output.contains("z"));
        assert!(output.contains("1000"));
    }
}
