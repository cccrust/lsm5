use std::sync::mpsc::{channel, Sender};
use std::thread;

#[allow(dead_code)]
pub enum BackgroundTask {
    Flush {
        entries: Vec<(Vec<u8>, crate::memtable::Value)>,
        seq: u64,
    },
    Compact {
        inputs: Vec<crate::sstable::SsTableMeta>,
        target_level: usize,
    },
}

#[allow(dead_code)]
pub struct BackgroundWorker {
    task_tx: Option<Sender<BackgroundTask>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl BackgroundWorker {
    pub fn new(_num_threads: usize) -> Self {
        let (task_tx, task_rx) = channel();
        let handle = thread::spawn(move || {
            for task in task_rx {
                match task {
                    BackgroundTask::Flush { entries: _, seq: _ } => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    BackgroundTask::Compact {
                        inputs: _,
                        target_level: _,
                    } => {
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            }
        });

        BackgroundWorker {
            task_tx: Some(task_tx),
            handle: Some(handle),
        }
    }

    pub fn submit_flush(&self, entries: Vec<(Vec<u8>, crate::memtable::Value)>, seq: u64) {
        if let Some(ref tx) = self.task_tx {
            let _ = tx.send(BackgroundTask::Flush { entries, seq });
        }
    }

    pub fn submit_compact(&self, inputs: Vec<crate::sstable::SsTableMeta>, target_level: usize) {
        if let Some(ref tx) = self.task_tx {
            let _ = tx.send(BackgroundTask::Compact {
                inputs,
                target_level,
            });
        }
    }
}

impl Drop for BackgroundWorker {
    fn drop(&mut self) {
        drop(self.task_tx.take());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_background_worker() {
        let worker = BackgroundWorker::new(1);
        worker.submit_flush(vec![], 1);
    }
}
