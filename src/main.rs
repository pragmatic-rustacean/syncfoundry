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
    // black_box(&tlang);
    println!("Escaped black box");
    let timer = Instant::now();
    thread::scope(|th| {
        println!("Inside scoped threads");
        for _ in 0..=5 {
            println!("Inside for loop");
            th.spawn(|| {
                println!("Inside a spawned thread");
                for _ in 0..5_000 {
                    println!("Inside the for loop inside the spawned thread");
                    *tlang.lock() += 1;
                }
            });
        }
    });
    println!("Locked {} times in {:?}", *tlang.lock(), timer.elapsed())
}
