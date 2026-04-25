//! Fixed-ish, VecDeque'ish buffer thingy'ish maybe'ish of use'ish somewhere… or then not.

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
