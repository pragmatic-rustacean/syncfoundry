#![allow(unused)]
use std::{hint::black_box, thread, time::Instant};

use crate::{channels::Channel, locks::mutx::LockMate};

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

    let mut mutx = LockMate::new(0);
    black_box(&mutx);
    let timer = Instant::now();
    for _ in 0..=5_000_000 {
        *mutx.lock() += 1;
    }
    println!("Locked {} times in {:?}", *mutx.lock(), timer.elapsed());

    // More impl
    let mut tlang = LockMate::new(0);
    black_box(&tlang);
    let timer = Instant::now();
    let scope = thread::scope(|th| {
        for _ in 0..=5 {
            th.spawn(|| {
                for _ in 0..5_000_000 {
                    *tlang.lock() += 1;
                }
            });
        }
    });
    println!("Locked {} times in {:?}", *tlang.lock(), timer.elapsed())
}
