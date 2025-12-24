#![allow(unused)]

use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem::MaybeUninit,
    sync::{
        Arc,
        atomic::{
            AtomicBool,
            Ordering::{Acquire, Relaxed, Release},
        },
    },
    thread::{self, Thread},
};

pub struct Channel<T> {
    message: UnsafeCell<MaybeUninit<T>>,
    ready: AtomicBool,
}

unsafe impl<T> Sync for Channel<T> where T: Send {}
impl<T> Drop for Channel<T> {
    fn drop(&mut self) {
        if *self.ready.get_mut() {
            unsafe {
                self.message.get_mut().assume_init_drop();
            }
        }
    }
}

impl<T> Channel<T> {
    pub fn new() -> Self {
        Self {
            message: UnsafeCell::new(MaybeUninit::uninit()),
            ready: AtomicBool::new(false),
        }
    }
    pub fn split<'a>(&'a mut self) -> (Sender<'a, T>, Receiver<'a, T>) {
        *self = Self::new();
        (
            Sender {
                channel: self,
                recv_thread: thread::current(),
            },
            Receiver {
                channel: self,
                _no_send: PhantomData,
            },
        )
    }
}

pub struct Sender<'a, T> {
    channel: &'a Channel<T>,
    recv_thread: Thread,
}

impl<'a, T> Sender<'a, T> {
    pub fn send(&self, msg: T) {
        unsafe {
            (*self.channel.message.get()).write(msg);
        };
        self.channel.ready.store(true, Release);
        self.recv_thread.unpark();
    }
}

pub struct Receiver<'a, T> {
    channel: &'a Channel<T>,
    _no_send: PhantomData<*const ()>,
}

impl<'a, T> Receiver<'a, T> {
    pub fn is_ready(&self) -> bool {
        self.channel.ready.load(Relaxed)
    }

    pub fn receive(&self) -> T {
        while !self.channel.ready.swap(false, Acquire) {
            thread::park();
        }
        unsafe { (*self.channel.message.get()).assume_init_read() }
    }
}
