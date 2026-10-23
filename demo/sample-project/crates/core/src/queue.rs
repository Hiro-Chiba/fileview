use std::collections::BinaryHeap;
use std::cmp::Ordering;

struct Entry<T> {
    priority: u8,
    seq: u64,
    item: T,
}

impl<T> PartialEq for Entry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.seq == other.seq
    }
}
impl<T> Eq for Entry<T> {}
impl<T> PartialOrd for Entry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<T> Ord for Entry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.seq.cmp(&self.seq))
    }
}

/// Priority queue that keeps FIFO order within the same priority.
pub struct JobQueue<T> {
    heap: BinaryHeap<Entry<T>>,
    seq: u64,
}

impl<T> JobQueue<T> {
    pub fn with_capacity(cap: usize) -> Self {
        Self { heap: BinaryHeap::with_capacity(cap), seq: 0 }
    }

    pub fn push(&mut self, priority: u8, item: T) {
        self.seq += 1;
        self.heap.push(Entry { priority, seq: self.seq, item });
    }

    pub fn pop(&mut self) -> Option<T> {
        self.heap.pop().map(|e| e.item)
    }
}
