//! Fixed-ish, VecDeque'ish buffer thingy'ish.

use std::collections::VecDeque;

pub struct IntentQueue<T> {
    inner: VecDeque<T>,
    limit: usize,
}

impl <T> IntentQueue<T> {
    pub fn push(&mut self, item: T) {
        if self.inner.len() >= self.limit {
            self.inner.pop_back();
        }
        self.inner.push_back(item);
    }

    pub fn pop(&mut self) -> Option<T> {
        self.inner.pop_front()
    }
}
