use std::thread;

use crate::locks::SpinLock;

mod channels;
mod locks;
mod pointers;

fn main() {
    let spin: SpinLock<Vec<i32>> = SpinLock::new(Vec::new());

    thread::scope(|s| {
        s.spawn(|| {
            spin.lock().push(3);
        });
        s.spawn(|| {
            let mut value = spin.lock();
            value.push(2);
            value.push(1);
            value.push(4);
        });
    });
    let g = spin.lock();
    
    assert!(g.as_slice() == [3, 2, 1, 4] || g.as_slice() == [2, 1, 4, 3]);
}
