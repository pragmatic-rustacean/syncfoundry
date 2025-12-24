use std::thread;

use crate::channels::Channel;
// use crate::locks::SpinLock;

mod channels;
mod locks;
mod pointers;

fn main() {
    let channel = Channel::new();
    let t = thread::current();

    thread::scope(|s| {
        s.spawn(|| {
            channel.send("Hello, my dear James");
            t.unpark();
        });

        while !channel.is_ready() {
            thread::park();
        }
    });
    println!("Results: {}", channel.receive())
}
