#![allow(unused)]

use std::{
    collections::VecDeque,
    sync::{Condvar, Mutex},
};

struct Channel<T> {
    queue: Mutex<VecDeque<T>>,
    ready: Condvar,
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            ready: Condvar::new(),
        }
    }

    pub fn send(&self, message: T) {
        self.queue.lock().unwrap().push_back(message);
        self.ready.notify_one();
    }

    pub fn receive(&self) -> T {
        let mut data = self.queue.lock().unwrap();
        loop {
            if let Some(message) = data.pop_front() {
                return message;
            }
            data = self.ready.wait(data).unwrap()
        }
    }
}
