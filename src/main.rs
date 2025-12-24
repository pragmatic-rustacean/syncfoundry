use std::thread;

use crate::channels::Channel;

mod channels;
mod locks;
mod pointers;

fn main() {
    let mut channel = Channel::new();
    thread::scope(|s| {
        let (sender, receiver) = channel.split();
        s.spawn(move || {
            sender.send("Hello, my dear James");
        });
        println!("Hey, {}", receiver.receive())
    });
}
