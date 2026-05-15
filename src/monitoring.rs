use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;

pub struct MonitoringServer {
    port: u16,
    running: bool,
    reads: Arc<AtomicU64>,
    writes: Arc<AtomicU64>,
    flushes: Arc<AtomicU64>,
    compactions: Arc<AtomicU64>,
    cache_hits: Arc<AtomicU64>,
    cache_misses: Arc<AtomicU64>,
}

impl MonitoringServer {
    pub fn new(port: u16) -> Self {
        MonitoringServer {
            port,
            running: false,
            reads: Arc::new(AtomicU64::new(0)),
            writes: Arc::new(AtomicU64::new(0)),
            flushes: Arc::new(AtomicU64::new(0)),
            compactions: Arc::new(AtomicU64::new(0)),
            cache_hits: Arc::new(AtomicU64::new(0)),
            cache_misses: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn start(&mut self) {
        if self.running {
            return;
        }
        self.running = true;
        let port = self.port;
        let reads = Arc::clone(&self.reads);
        let writes = Arc::clone(&self.writes);
        let flushes = Arc::clone(&self.flushes);
        let compactions = Arc::clone(&self.compactions);
        let cache_hits = Arc::clone(&self.cache_hits);
        let cache_misses = Arc::clone(&self.cache_misses);

        thread::spawn(move || {
            let addr = format!("0.0.0.0:{}", port);
            if let Ok(listener) = TcpListener::bind(&addr) {
                for mut stream in listener.incoming().flatten() {
                    Self::handle_request(
                        &reads,
                        &writes,
                        &flushes,
                        &compactions,
                        &cache_hits,
                        &cache_misses,
                        &mut stream,
                    );
                }
            }
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_request(
        reads: &AtomicU64,
        writes: &AtomicU64,
        flushes: &AtomicU64,
        compactions: &AtomicU64,
        cache_hits: &AtomicU64,
        cache_misses: &AtomicU64,
        stream: &mut std::net::TcpStream,
    ) {
        let mut buffer = [0; 1024];
        if stream.read(&mut buffer).is_err() {
            return;
        }

        let request = String::from_utf8_lossy(&buffer);
        let (status, body) = if request.starts_with("GET /stats ") {
            (
                "200 OK",
                Self::json_stats(
                    reads,
                    writes,
                    flushes,
                    compactions,
                    cache_hits,
                    cache_misses,
                ),
            )
        } else if request.starts_with("GET /cache ") {
            ("200 OK", Self::json_cache(cache_hits, cache_misses))
        } else if request.starts_with("GET /health ") {
            ("200 OK", r#"{"status":"ok"}"#.to_string())
        } else if request.starts_with("GET / ") {
            (
                "200 OK",
                r#"{"endpoints":["/stats","/cache","/health"]}"#.to_string(),
            )
        } else {
            ("404 Not Found", r#"{"error":"not found"}"#.to_string())
        };

        let response = format!(
            "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            status,
            body.len(),
            body
        );
        let _ = stream.write_all(response.as_bytes());
    }

    #[allow(clippy::too_many_arguments)]
    fn json_stats(
        reads: &AtomicU64,
        writes: &AtomicU64,
        flushes: &AtomicU64,
        compactions: &AtomicU64,
        cache_hits: &AtomicU64,
        cache_misses: &AtomicU64,
    ) -> String {
        let reads = reads.load(Ordering::Relaxed);
        let writes = writes.load(Ordering::Relaxed);
        let flushes = flushes.load(Ordering::Relaxed);
        let compactions = compactions.load(Ordering::Relaxed);
        let cache_hits = cache_hits.load(Ordering::Relaxed);
        let cache_misses = cache_misses.load(Ordering::Relaxed);

        let hit_rate = if cache_hits + cache_misses > 0 {
            cache_hits as f64 / (cache_hits + cache_misses) as f64
        } else {
            0.0
        };

        format!(
            r#"{{"operations":{{"reads":{},"writes":{},"flushes":{},"compactions":{}}},"cache":{{"hits":{},"misses":{},"hit_rate":{:.2}}}}}"#,
            reads, writes, flushes, compactions, cache_hits, cache_misses, hit_rate
        )
    }

    fn json_cache(cache_hits: &AtomicU64, cache_misses: &AtomicU64) -> String {
        let hits = cache_hits.load(Ordering::Relaxed);
        let misses = cache_misses.load(Ordering::Relaxed);
        let hit_rate = if hits + misses > 0 {
            hits as f64 / (hits + misses) as f64
        } else {
            0.0
        };
        format!(
            r#"{{"hits":{},"misses":{},"hit_rate":{:.2}}}"#,
            hits, misses, hit_rate
        )
    }

    pub fn record_read(&self, hit: bool) {
        if hit {
            self.cache_hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.cache_misses.fetch_add(1, Ordering::Relaxed);
        }
        self.reads.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_write(&self) {
        self.writes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_flush(&self) {
        self.flushes.fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_compaction(&self) {
        self.compactions.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_server() {
        let server = MonitoringServer::new(19001);
        let _ = server; // Just check it compiles
    }
}
