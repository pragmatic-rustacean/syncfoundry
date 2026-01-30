use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicU32,
    u32,
};

use atomic_wait::{wait, wake_all, wake_one};

pub struct Raulock<T> {
    // The number of readers, or u32::MAX if write-locked
    state: AtomicU32,
    // Increment to wake up writers.
    write_wake_counter: AtomicU32,
    value: UnsafeCell<T>,
}

unsafe impl<T> Sync for Raulock<T> where T: Send + Sync {}

pub struct ReadGuard<'a, T> {
    rwlock: &'a Raulock<T>,
}

impl<'a, T> Deref for ReadGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<'a, T> Drop for ReadGuard<'a, T> {
    fn drop(&mut self) {
        if self
            .rwlock
            .state
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed)
            == 1
        {
            self.rwlock
                .write_wake_counter
                .fetch_add(1, std::sync::atomic::Ordering::Release);
            wake_one(&self.rwlock.write_wake_counter);
        }
    }
}

pub struct WriteGuard<'a, T> {
    rwlock: &'a Raulock<T>,
}

impl<'a, T> Deref for WriteGuard<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.rwlock.value.get() }
    }
}

impl<'a, T> DerefMut for WriteGuard<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { &mut *self.rwlock.value.get() }
    }
}

impl<'a, T> Drop for WriteGuard<'a, T> {
    fn drop(&mut self) {
        self.rwlock
            .write_wake_counter
            .store(0, std::sync::atomic::Ordering::Release);
        self.rwlock
            .write_wake_counter
            .fetch_add(1, std::sync::atomic::Ordering::Release);

        wake_one(&self.rwlock.write_wake_counter);
        wake_all(&self.rwlock.state);
    }
}

impl<T> Raulock<T> {
    pub fn new(value: T) -> Self {
        Self {
            state: AtomicU32::new(0),
            value: UnsafeCell::new(value),
            write_wake_counter: AtomicU32::new(0),
        }
    }
    pub fn read(&self) -> ReadGuard<'_, T> {
        let mut state = self.state.load(std::sync::atomic::Ordering::Relaxed);
        'read: loop {
            if state < u32::MAX {
                assert!(state != u32::MAX - 1, "Too many readers");
                match self.state.compare_exchange(
                    state,
                    state + 1,
                    std::sync::atomic::Ordering::Acquire,
                    std::sync::atomic::Ordering::Relaxed,
                ) {
                    Ok(_) => return ReadGuard { rwlock: self },
                    Err(e) => state = e,
                }
            }
            if state == u32::MAX {
                wait(&self.state, u32::MAX);
                state = self.state.load(std::sync::atomic::Ordering::Relaxed)
            }
        }
    }
    pub fn write(&mut self) -> WriteGuard<'_, T> {
        while self
            .state
            .compare_exchange(
                0,
                u32::MAX,
                std::sync::atomic::Ordering::Acquire,
                std::sync::atomic::Ordering::Relaxed,
            )
            .is_err()
        {
            let writer = self
                .write_wake_counter
                .load(std::sync::atomic::Ordering::Acquire);
            if self.state.load(std::sync::atomic::Ordering::Acquire) != 0 {
                // Wait if the Rollock is still locked, but only if there have been no wake signals since we checked.
                wait(&self.state, writer);
            }
        }

        WriteGuard { rwlock: self }
    }
}
