# ██░░ nanoprogress

[![Crates.io](https://img.shields.io/crates/v/nanoprogress)](https://crates.io/crates/nanoprogress)
[![Docs.rs](https://docs.rs/nanoprogress/badge.svg)](https://docs.rs/nanoprogress/latest/nanoprogress/)

A minimal, zero-dependency terminal progress bar for Rust CLI applications.

![demo](demo.gif)

Inspired by the [nanospinner](https://github.com/anthonysgro/nanospinner) package, `nanoprogress` gives you a lightweight determinate progress bar using only the Rust standard library — no heavy crates, no transitive dependencies, under 300 lines of code.

## Nano Crate Family

Part of the nano crate family — minimal, zero-dependency building blocks for CLI apps in Rust:

- [nanocolor](https://github.com/anthonysgro/nanocolor) — terminal colors and styles
- [nanospinner](https://github.com/anthonysgro/nanospinner) — terminal spinners
- [nanoprogress](https://github.com/anthonysgro/nanoprogress) — progress bars
- [nanologger](https://github.com/anthonysgro/nanologger) — minimal logger
- [nanotime](https://github.com/anthonysgro/nanotime) — time utilities

## Motivation

Most Rust progress bar crates (like `indicatif`) are feature-rich but pull in multiple dependencies, increasing compile times and binary size. If all you need is a simple progress bar with a count, percentage, and success/failure states, those crates are overkill.

`nanoprogress` solves this by providing the essentials and nothing more:

- Zero external dependencies (only `std`)
- Tiny footprint (< 300 LOC)
- Simple, ergonomic builder API
- Thread-safe — share across threads with `Clone`
- Automatic cleanup via `Drop`

## Comparison

| Crate | Dependencies | Lines of Code | Clean Build Time | Thread-safe |
|-------|-------------|---------------|------------------|-------------|
| `nanoprogress` | 0 | ~200 | ~0.5s | Yes |
| `progress_bar` | 0 | ~430 | ~3.4s | No |
| `pbr` | 3 | ~730 | ~4.3s | No |
| `linya` | 1 | ~180 | ~3.8s | Yes (Mutex) |
| `kdam` | 3+ | ~2,000 | ~2.2s | Yes |
| `indicatif` | 5+ | ~4,500 | ~1.9s | Yes |

Build times measured from a clean `cargo build --release` on macOS aarch64 (Apple Silicon). Your numbers may vary by platform.

`nanoprogress` is for when you want a single progress bar with zero compile-time cost and nothing else. If you need multi-bars, templates, or ETA estimation, reach for `indicatif` or `kdam`.

## Features

- Determinate progress bar with fill/empty characters (`█░`)
- Percentage display and current/total count
- Colored finalization: green `✔` for success, red `✖` for failure
- Customizable bar width, fill character, and empty character
- Update the message while the bar is running
- Custom writer support (stdout, stderr, or any `io::Write + Send`)
- Automatic cleanup via `Drop` — no dangling cursor if you forget to finalize
- Automatic TTY detection — ANSI codes are skipped when output is piped or redirected

## Quick Start

```bash
cargo add nanoprogress
```

```rust
use nanoprogress::ProgressBar;
use std::thread;
use std::time::Duration;

fn main() {
    let bar = ProgressBar::new(100)
        .message("Downloading...")
        .start();

    for _ in 0..100 {
        thread::sleep(Duration::from_millis(30));
        bar.tick(1);
    }
    bar.success("Download complete");
}
```

## Usage

### Create and start a progress bar

```rust
let bar = ProgressBar::new(100)
    .message("Processing...")
    .start();
```

### Increment progress

```rust
bar.tick(1);   // increment by 1
bar.tick(10);  // increment by 10 — clamped to total
```

### Finalize with success or failure

```rust
bar.success("All done");    // ✔ All done
bar.fail("Something broke"); // ✖ Something broke
```

### Update the message mid-progress

```rust
let bar = ProgressBar::new(200).message("Step 1...").start();
for i in 0..200 {
    if i == 100 {
        bar.set_message("Step 2...");
    }
    bar.tick(1);
}
bar.success("Complete");
```

### Customize the bar appearance

```rust
let bar = ProgressBar::new(50)
    .width(30)
    .fill('#')
    .empty('-')
    .message("Installing...")
    .start();
```

### Write to a custom destination

```rust
use std::io;

let bar = ProgressBar::new(100)
    .writer(Box::new(io::stderr()))
    .start();
```

### Share across threads

```rust
use std::thread;

let bar = ProgressBar::new(100).start();

let handles: Vec<_> = (0..4).map(|_| {
    let bar = bar.clone();
    thread::spawn(move || {
        for _ in 0..25 {
            bar.tick(1);
        }
    })
}).collect();

for h in handles { h.join().unwrap(); }
bar.success("Done");
```

### Piped / non-TTY output

When stdout isn't a terminal (e.g. piped to a file or another program), `nanoprogress` automatically skips ANSI codes and prints each update on a new line:

```bash
$ my_tool | cat
[░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░]   0% 0/100 Downloading...
[████████████████████░░░░░░░░░░░░░░░░░░░░]  50% 50/100 Downloading...
✔ Download complete
```

No configuration needed — custom writers default to non-TTY mode. To force TTY behavior:

```rust
let bar = ProgressBar::new(100)
    .writer(my_writer)
    .tty(true)
    .start();
```

## Contributing

Contributions are welcome. To get started:

1. Fork the repository
2. Create a feature branch (`git checkout -b my-feature`)
3. Make your changes
4. Run the tests: `cargo test`
5. Submit a pull request

Please keep changes minimal and focused. This crate's goal is to stay small and dependency-free.

## License

This project is licensed under the [MIT License](LICENSE).
