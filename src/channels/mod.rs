#![allow(unused)]

use std::{
    cell::UnsafeCell,
    collections::VecDeque,
    mem::MaybeUninit,
    sync::{
        Condvar, Mutex,
        atomic::{
            AtomicBool,
            Ordering::{Acquire, Relaxed, Release},
        },
    },
};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    is_ready: AtomicBool,
    in_use: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.is_ready.get_mut() {
            unsafe {
                self.message.get_mut().assume_init_drop();
            }
        }
    }
}

impl<T> Channel<T> {
    pub const fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            is_ready: AtomicBool::new(false),
            in_use: AtomicBool::new(false),
        }
    }

    /// Safety: Only call this once!
    pub fn send(&self, message: T) {
        if self.in_use.swap(true, Relaxed) {
            panic!("Can't send more than one message.")
        }
        unsafe {
            (*self.message.get()).write(message);
        };
        self.is_ready.store(true, Release);
    }

    pub fn is_ready(&self) -> bool {
        self.is_ready.load(Relaxed)
    }

    /// Safety: Only call this once, and only after is_ready returns true.
    pub fn receive(&self) -> T {
        if !self.is_ready.swap(false, Acquire) {
            panic!("No message yet!!")
        }
        unsafe { (*self.message.get()).assume_init_read() }
    }
}
