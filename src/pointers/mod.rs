#![allow(unused)]

use std::{
    cell::UnsafeCell,
    ops::Deref,
    process::abort,
    ptr::NonNull,
    sync::atomic::{
        AtomicUsize,
        Ordering::{Acquire, Relaxed, Release},
        fence,
    },
};

struct ArcData<T> {
    data_ref_count: AtomicUsize,
    alloc_ref_count: AtomicUsize,
    data: UnsafeCell<Option<T>>,
}

pub struct Arc<T> {
    weak: Weak<T>,
}

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

            return Some(Arc { weak: self.clone() });
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

// Arc implementations...
impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let ptr = self.weak.data().data.get();
        unsafe { (*ptr).as_ref().unwrap() }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().data_ref_count.fetch_sub(1, Release) == 1 {
            fence(Acquire);
            let ptr = self.data().data.get();
            unsafe {
                (*ptr) = None;
            };
        }
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        let weak = self.weak.clone();
        if weak.data().data_ref_count.fetch_add(1, Relaxed) > usize::MAX {
            abort();
        }
        Self { weak }
    }
}

impl<T> Arc<T> {
    pub fn new(data: T) -> Self {
        Self {
            weak: Weak {
                ptr: NonNull::from(Box::leak(Box::new(ArcData {
                    data_ref_count: AtomicUsize::new(1),
                    alloc_ref_count: AtomicUsize::new(1),
                    data: UnsafeCell::new(Some(data)),
                }))),
            },
        }
    }
    fn data(&self) -> &ArcData<T> {
        unsafe { self.weak.ptr.as_ref() }
    }

    pub fn get_mut(arc: &mut Self) -> Option<&mut T> {
        if arc.data().alloc_ref_count.load(Relaxed) == 1 {
            fence(Acquire);
            let arc_data = unsafe { arc.weak.ptr.as_mut() };
            let option = arc_data.data.get_mut();

            let data = option.as_mut().unwrap();
            Some(data)
        } else {
            None
        }
    }

    pub fn downgrade(arc: &Self) -> Weak<T> {
        arc.weak.clone()
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
