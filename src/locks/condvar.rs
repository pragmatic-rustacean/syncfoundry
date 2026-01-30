#![allow(unused)]

use std::{
    marker::PhantomData,
    sync::atomic::{AtomicU32, AtomicUsize},
};

use atomic_wait::{wait, wake_all, wake_one};

use crate::locks::mutx::LockGuard;
pub struct Condvar<T> {
    counter: AtomicU32,
    num_waiters: AtomicUsize,
    _data: PhantomData<T>,
}

impl<T> Condvar<T> {
    pub fn new() -> Self {
        Self {
            counter: AtomicU32::new(0),
            _data: PhantomData,
            num_waiters: AtomicUsize::new(0),
        }
    }

    pub fn notify_one(&self) {
        if self.num_waiters.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            wake_one(&self.counter);
        }
    }

    pub fn notify_all(&self) {
        if self.num_waiters.load(std::sync::atomic::Ordering::Relaxed) > 0 {
            self.counter
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            wake_all(&self.counter);
        }
    }

    pub fn wait<'a>(&self, guard: LockGuard<'a, T>) -> LockGuard<'a, T> {
        self.num_waiters
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let counter_value = self.counter.load(std::sync::atomic::Ordering::Relaxed);
        let mutx = guard.mutx;
        // Unlock the mutx by dropping the guard, but remember the mutx so that we can lock it.
        drop(guard);
        // Wait, but only if the counter hasn't changed since unlocking.
        wait(&self.counter, counter_value);
        self.num_waiters
            .fetch_sub(1, std::sync::atomic::Ordering::Relaxed);

        mutx.lock()
    }
}

#[cfg(test)]
mod test {
    use std::{thread, time::Duration};

    use crate::locks::{condvar::Condvar, mutx::LockMate};
    #[test]
    fn test_condvar() {
        let mtx = LockMate::new(0);
        let condv: Condvar<i32> = Condvar::new();

        let mut wakeups = 0;
        thread::scope(|th| {
            th.spawn(|| {
                thread::sleep(Duration::from_millis(300));
                *mtx.lock() = 123;
                condv.notify_one();
            });

            let mut m = mtx.lock();
            while *m < 100 {
                m = condv.wait(m);
                wakeups += 1;
            }

            assert_eq!(*m, 123);
        });

        assert!(wakeups < 10);
    }
}
