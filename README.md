## Atomics and Locks

This little crate is a playground for low–level concurrency experiments in Rust. It is not meant to be a polished library; it is a place to poke at atomics, write custom locks, and get an intuition for what the CPU is actually doing when you share data across threads.

The main focus right now is a hand‑rolled mutex (`LockMate`) built on top of atomics and the `atomic-wait` crate, plus a few small examples that hammer on it from multiple threads.

### What this project does

- **Custom lock (`LockMate`)**: A mutex–like type implemented using `AtomicU32`, `UnsafeCell`, and `atomic-wait`.
- **Spin + park strategy**: Fast path tries to grab the lock, then spins briefly, then parks the thread when things are really contended.
- **Simple benchmark demo**: `main.rs` runs a couple of tight loops that repeatedly lock, bump an integer, and print out how long it took.
- **Channel experiments (WIP)**: There is a `Channel` type wired up in `main.rs` for message passing; this is also experimental territory.

None of this is “production ready”. The point is to understand the trade‑offs and behaviour of atomics and locks by writing them from scratch.

### How to build and run

From the `atomics_and_locks` directory:

```bash
cargo build
cargo run
```

You should see:

- A short message sent through the custom channel.
- Timing information for how long it took to perform millions of lock/unlock cycles on `LockMate`, both in a single thread and from several threads in parallel.

If you want to play with it, change the loop counts, add more threads, or tweak the locking strategy and watch how the timings move.

### Project layout

- **`src/main.rs`**: Entry point. Wires up the channel demo and runs the lock benchmarks.
- **`src/locks/mutx.rs`**: Implementation of `LockMate` and its guard type. This is where the atomic protocol and wake/wait logic live.
- **`src/locks/condvar.rs`**: Placeholder for condition‑variable style primitives; currently empty and waiting for ideas.
- **`src/channels` / `src/pointers`**: Additional experiments around message passing and pointer/ownership tricks (work in progress).

### Why this exists

Rust gives you great high‑level concurrency tools, but it’s very easy to treat them as magic. This repo is here to peel back a layer:

- What does a mutex really do under the hood?
- How much spinning is “enough” before you let the OS park your thread?
- How expensive is it to bounce a shared counter across several threads?

If you’re curious about this sort of thing, clone the ideas, break them, and make them better. That’s the whole point.

