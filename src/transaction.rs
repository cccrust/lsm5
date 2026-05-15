use crate::error::Result;

#[derive(Clone, Debug, PartialEq)]
pub enum Operation {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
}

#[derive(Clone, Debug)]
pub struct Transaction {
    operations: Vec<Operation>,
    committed: bool,
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            committed: false,
        }
    }

    pub fn put(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.operations.push(Operation::Put(key, value));
    }

    pub fn delete(&mut self, key: Vec<u8>) {
        self.operations.push(Operation::Delete(key));
    }

    pub fn commit(&mut self) {
        self.committed = true;
    }

    pub fn is_committed(&self) -> bool {
        self.committed
    }

    pub fn rollback(&mut self) {
        self.operations.clear();
    }

    pub fn apply_to_memtable(&self, memtable: &mut crate::memtable::MemTable) -> Result<()> {
        for op in &self.operations {
            match op {
                Operation::Put(key, value) => memtable.put(key.clone(), value.clone()),
                Operation::Delete(key) => memtable.delete(key.clone()),
            }
        }
        Ok(())
    }

    pub fn apply_to_wal(&self, wal: &mut crate::wal::Wal) -> Result<()> {
        for op in &self.operations {
            match op {
                Operation::Put(key, value) => wal.append_put(key, value)?,
                Operation::Delete(key) => wal.append_delete(key)?,
            }
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.operations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}
