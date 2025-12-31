#![allow(unused)]

use std::{
    cell::UnsafeCell,
    mem::ManuallyDrop,
    ops::Deref,
    process::abort,
    ptr::NonNull,
    sync::atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
        fence,
    },
    usize,
};

struct ArcData<T> {
    // Number of Arc's
    data_ref_count: AtomicUsize,
    // Number of Weak's, plus 1 if there are any Arc's
    alloc_ref_count: AtomicUsize,
    // The data. Dropped if there are only Weak's left.
    data: UnsafeCell<ManuallyDrop<T>>,
}

pub struct Arc<T> {
    ptr: NonNull<ArcData<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

pub struct Weak<T> {
    ptr: NonNull<ArcData<T>>,
}

// Weak implementation...
unsafe impl<T> Send for Weak<T> where T: Send + Sync {}
unsafe impl<T> Sync for Weak<T> where T: Send + Sync {}

impl<T> Weak<T> {
    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn upgrade(&self) -> Option<Arc<T>> {
        let mut data_count = self.data().data_ref_count.load(Relaxed);
        loop {
            if data_count == 0 {
                return None;
            }
            assert!(data_count < usize::MAX);
            if let Err(err) = self.data().data_ref_count.compare_exchange(
                data_count,
                data_count + 1,
                Relaxed,
                Relaxed,
            ) {
                data_count = err;
                continue;
            }

            return Some(Arc { ptr: self.ptr });
        }
    }
}

impl<T> Clone for Weak<T> {
    fn clone(&self) -> Self {
        if self.data().alloc_ref_count.fetch_add(1, Relaxed) > usize::MAX / 2 {
            abort();
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Drop for Weak<T> {
    fn drop(&mut self) {
        if self.data().alloc_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                drop(Box::from_raw(self.ptr.as_ptr()));
            }
        }
    }
}

// Arc implementations...
impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data().data.get() }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().data_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            unsafe {
                ManuallyDrop::drop(&mut *self.data().data.get());
            };
            drop(Weak { ptr: self.ptr });
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        if self.data().data_ref_count.fetch_add(1, Relaxed) > usize::MAX {
            abort();
        }
        Self { ptr: self.ptr }
    }
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Self {
            ptr: NonNull::from(Box::leak(Box::new(ArcData {
                data_ref_count: AtomicUsize::new(1),
                alloc_ref_count: AtomicUsize::new(1),
                data: UnsafeCell::new(ManuallyDrop::new(data)),
            }))),
        }
    }
    fn data(&self) -> &ArcData<T> {
        unsafe { self.ptr.as_ref() }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc
            .data()
            .alloc_ref_count
            .compare_exchange(1, usize::MAX, Acquire, Relaxed)
            .is_err()
        {
            return None;
        }

        let is_unique = arc.data().data_ref_count.load(Relaxed) == 1;
        arc.data().alloc_ref_count.store(1, Release);
        if !is_unique {
            return None;
        }

        fence(Acquire);

        unsafe { Some(&mut *arc.data().data.get()) }
    }

    pub fn downgrade(arc: &Self) -> Weak<T> {
        let mut n = arc.data().alloc_ref_count.load(Relaxed);
        loop {
            if n == usize::MAX {
                std::hint::spin_loop();
                n = arc.data().alloc_ref_count.load(Relaxed);
                continue;
            }
            assert!(n < usize::MAX - 1);
            if let Err(err) =
                arc.data()
                    .alloc_ref_count
                    .compare_exchange(n, n + 1, Acquire, Relaxed)
            {
                n = err;
                continue;
            }
            return Weak {
                ptr: arc.ptr.clone(),
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{sync::atomic::AtomicUsize, thread};

    #[test]
    fn test_pointer() {
        static NUM_DROP: AtomicUsize = AtomicUsize::new(0);
        struct DetectDrop;
        impl Drop for DetectDrop {
            fn drop(&mut self) {
                NUM_DROP.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            }
        }

        let x = Arc::new(("I am a man", DetectDrop));
        let y = Arc::downgrade(&x);
        let z = Arc::downgrade(&x);

        let t = thread::spawn(move || {
            let x = y.upgrade().unwrap();
            assert_eq!(x.0, "I am a man");
        });

        assert_eq!(x.0, "I am a man");
        t.join().unwrap();

        assert_eq!(NUM_DROP.load(Acquire), 0);
        assert!(z.upgrade().is_some());
        drop(x);
        assert_eq!(NUM_DROP.load(Acquire), 1);
        assert!(z.upgrade().is_none())
    }
}
