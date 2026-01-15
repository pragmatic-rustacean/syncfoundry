#![allow(unused)]

use std::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{
        AtomicU32,
        Ordering::{Acquire, Relaxed, Release},
    },
};

use atomic_wait::{wait, wake_all, wake_one};

pub(crate) struct LockMate<T> {
    /// 0: Unlocked.
    /// 1: Locked but not thread waiting.
    /// 2: Locked but threads waiting.
    pub(crate) state: AtomicU32,
    pub(crate) data: UnsafeCell<T>,
}

pub(crate) struct LockGuard<'a, T> {
    pub(crate) mutx: &'a LockMate<T>,
}

unsafe impl<T> Sync for LockMate<T> where T: Send {}

impl<'a, T> Deref for LockGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.mutx.data.get() }
    }
}

impl<'a, T> DerefMut for LockGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.mutx.data.get() }
    }
}

impl<'a, T> Drop for LockGuard<'a, T> {
    fn drop(&mut self) {
        if self.mutx.state.swap(0, Relaxed) == 2 {
            wake_one(&self.mutx.state);
        }
    }
}

impl<T> LockMate<T> {
    pub(crate) fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            data: UnsafeCell::new(value),
        }
    }

    pub(crate) fn lock(&'_ self) -> LockGuard<'_, T> {
        if self.state.compare_exchange(0, 1, Acquire, Relaxed).is_err() {
            while self.state.swap(2, Release) != 0 {
                self.lock_contended(&self.state);
            }
        }
        LockGuard { mutx: self }
    }

    #[cold]
    pub(self) fn lock_contended(&self, state: &AtomicU32) {
        let mut spin_count = 1;
        while state.load(Relaxed) == 1 && spin_count < 100 {
            spin_count += 1;
            spin_loop();
        }

        if state.compare_exchange(0, 1, Acquire, Relaxed).is_ok() {
            return;
        }

        while state.swap(2, Acquire) != 0 {
            wait(state, 2);
        }
    }
}
