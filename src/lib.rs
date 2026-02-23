//! A minimal, zero-dependency terminal progress bar for Rust CLI applications.
//!
//! # Quick Start
//!
//! ```no_run
//! use nanoprogress::ProgressBar;
//! use std::thread;
//! use std::time::Duration;
//!
//! let bar = ProgressBar::new(100)
//!     .message("Downloading...")
//!     .start();
//!
//! for _ in 0..100 {
//!     thread::sleep(Duration::from_millis(30));
//!     bar.tick(1);
//! }
//! bar.success("Download complete");
//! ```
//!
//! # Features
//!
//! - Zero external dependencies
//! - Thread-safe (`Send + Sync`) — clone and share across threads
//! - Automatic TTY detection — ANSI codes are skipped when output is piped
//! - Customizable bar width, fill/empty characters, and messages
//! - Clean finalization with colored `✔` / `✖` symbols
//! - Automatic cleanup via `Drop`

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

struct BarConfig {
    width: usize,
    fill: char,
    empty: char,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            width: 40,
            fill: '█',
            empty: '░',
        }
    }
}

struct ProgressBarState {
    current: u64,
    total: u64,
    message: String,
    finished: bool,
    writer: Box<dyn Write + Send>,
    config: BarConfig,
    is_tty: bool,
}

impl ProgressBarState {
    fn render(&mut self) {
        let ratio = self.current as f64 / self.total.max(1) as f64;
        let filled = (ratio * self.config.width as f64).round() as usize;
        let empty = self.config.width - filled;
        let percent = (ratio * 100.0) as u64;

        let bar: String = std::iter::repeat_n(self.config.fill, filled)
            .chain(std::iter::repeat_n(self.config.empty, empty))
            .collect();

        let line = if self.message.is_empty() {
            format!("[{}] {:>3}% {}/{}", bar, percent, self.current, self.total)
        } else {
            format!(
                "[{}] {:>3}% {}/{} {}",
                bar, percent, self.current, self.total, self.message
            )
        };

        if self.is_tty {
            write!(self.writer, "\r{}", line).ok();
        } else {
            writeln!(self.writer, "{}", line).ok();
        }
        self.writer.flush().ok();
    }

    fn finalize(&mut self, symbol: &str, color_code: &str, msg: &str) {
        if self.finished {
            return;
        }
        self.finished = true;

        if self.is_tty {
            write!(
                self.writer,
                "\r\x1b[2K{}{}\x1b[0m {}\n",
                color_code, symbol, msg
            )
            .ok();
        } else {
            writeln!(self.writer, "{} {}", symbol, msg).ok();
        }
        self.writer.flush().ok();
    }
}

// --- TTY Detection ---

#[cfg(unix)]
fn is_stdout_tty() -> bool {
    extern "C" {
        fn isatty(fd: std::os::raw::c_int) -> std::os::raw::c_int;
    }
    unsafe { isatty(1) != 0 }
}

#[cfg(windows)]
fn is_stdout_tty() -> bool {
    use std::os::windows::io::AsRawHandle;
    extern "system" {
        fn GetConsoleMode(handle: *mut std::ffi::c_void, mode: *mut u32) -> i32;
    }
    let handle = io::stdout().as_raw_handle();
    let mut mode: u32 = 0;
    unsafe { GetConsoleMode(handle as *mut _, &mut mode) != 0 }
}

#[cfg(not(any(unix, windows)))]
fn is_stdout_tty() -> bool {
    false
}

// --- Builder ---

/// Builder for configuring and starting a [`ProgressBar`].
///
/// Created via [`ProgressBar::new`]. Chain configuration methods and call
/// [`.start()`](ProgressBarBuilder::start) to begin rendering.
///
/// ```no_run
/// use nanoprogress::ProgressBar;
///
/// let bar = ProgressBar::new(100)
///     .width(30)
///     .fill('#')
///     .empty('-')
///     .message("Working...")
///     .start();
/// ```
pub struct ProgressBarBuilder {
    total: u64,
    config: BarConfig,
    message: String,
    writer: Option<Box<dyn Write + Send>>,
    tty_override: Option<bool>,
}

impl ProgressBarBuilder {
    /// Set the width of the bar track in characters. Default: 40.
    pub fn width(mut self, width: usize) -> Self {
        self.config.width = width;
        self
    }

    /// Set the fill character for completed progress. Default: `█`.
    pub fn fill(mut self, ch: char) -> Self {
        self.config.fill = ch;
        self
    }

    /// Set the empty character for remaining progress. Default: `░`.
    pub fn empty(mut self, ch: char) -> Self {
        self.config.empty = ch;
        self
    }

    /// Set an initial message displayed after the count.
    pub fn message(mut self, msg: &str) -> Self {
        self.message = msg.to_string();
        self
    }

    /// Direct output to a custom writer instead of stdout.
    /// Custom writers default to non-TTY mode unless overridden with [`.tty(true)`](ProgressBarBuilder::tty).
    pub fn writer(mut self, writer: Box<dyn Write + Send>) -> Self {
        self.writer = Some(writer);
        self
    }

    /// Explicitly set TTY mode, overriding auto-detection.
    pub fn tty(mut self, is_tty: bool) -> Self {
        self.tty_override = Some(is_tty);
        self
    }

    /// Build and start the progress bar, rendering the initial state immediately.
    pub fn start(self) -> ProgressBar {
        let total = if self.total == 0 { 1 } else { self.total };
        let has_custom_writer = self.writer.is_some();
        let writer = self.writer.unwrap_or_else(|| Box::new(io::stdout()));
        let is_tty = self.tty_override.unwrap_or_else(|| {
            if has_custom_writer {
                false
            } else {
                is_stdout_tty()
            }
        });

        let mut state = ProgressBarState {
            current: 0,
            total,
            message: self.message,
            finished: false,
            writer,
            config: self.config,
            is_tty,
        };
        state.render();

        ProgressBar {
            state: Arc::new(Mutex::new(state)),
        }
    }
}

// --- ProgressBar ---

/// A thread-safe terminal progress bar.
///
/// Create one via the builder API:
///
/// ```no_run
/// use nanoprogress::ProgressBar;
///
/// let bar = ProgressBar::new(100).start();
/// bar.tick(10);
/// bar.success("Done");
/// ```
///
/// `ProgressBar` is `Clone`, `Send`, and `Sync` — clone it to share across threads.
/// When the last reference is dropped without finalization, a newline is written
/// to leave the terminal in a clean state.
pub struct ProgressBar {
    state: Arc<Mutex<ProgressBarState>>,
}

impl ProgressBar {
    /// Create a new builder with the given total.
    /// A total of 0 is normalized to 1 to avoid division by zero.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(total: u64) -> ProgressBarBuilder {
        ProgressBarBuilder {
            total,
            config: BarConfig::default(),
            message: String::new(),
            writer: None,
            tty_override: None,
        }
    }

    /// Increment progress by `amount`, clamped to the total. Re-renders the bar.
    /// No-op if the bar has been finalized.
    pub fn tick(&self, amount: u64) {
        let mut s = self.state.lock().unwrap();
        if s.finished {
            return;
        }
        s.current = s.current.saturating_add(amount).min(s.total);
        s.render();
    }

    /// Update the displayed message. Takes effect on the next render.
    pub fn set_message(&self, msg: &str) {
        let mut s = self.state.lock().unwrap();
        s.message = msg.to_string();
    }

    /// Finalize with a green `✔` and the given message. Stops further ticks.
    pub fn success(&self, msg: &str) {
        let mut s = self.state.lock().unwrap();
        s.finalize("✔", "\x1b[32m", msg);
    }

    /// Finalize with a red `✖` and the given message. Stops further ticks.
    pub fn fail(&self, msg: &str) {
        let mut s = self.state.lock().unwrap();
        s.finalize("✖", "\x1b[31m", msg);
    }
}

impl Clone for ProgressBar {
    fn clone(&self) -> Self {
        ProgressBar {
            state: Arc::clone(&self.state),
        }
    }
}

impl Drop for ProgressBar {
    fn drop(&mut self) {
        if Arc::strong_count(&self.state) == 1 {
            if let Ok(mut s) = self.state.lock() {
                if !s.finished {
                    let _ = writeln!(s.writer);
                    let _ = s.writer.flush();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    // --- Task 4.1: Test helper ---

    #[derive(Clone)]
    struct TestWriter(Arc<Mutex<Vec<u8>>>);

    impl TestWriter {
        fn new() -> Self {
            TestWriter(Arc::new(Mutex::new(Vec::new())))
        }

        fn output(&self) -> String {
            String::from_utf8_lossy(&self.0.lock().unwrap()).to_string()
        }
    }

    impl io::Write for TestWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    fn make_writer() -> (TestWriter, Box<dyn Write + Send>) {
        let tw = TestWriter::new();
        let boxed: Box<dyn Write + Send> = Box::new(tw.clone());
        (tw, boxed)
    }

    // --- Task 4.2: Builder defaults and custom writer ---

    #[test]
    fn test_builder_defaults() {
        let (tw, w) = make_writer();
        let _bar = ProgressBar::new(100).writer(w).start();
        let out = tw.output();
        // Default bar: 40 chars of empty char, 0%, 0/100
        assert!(
            out.contains("0% 0/100"),
            "expected default render, got: {out}"
        );
    }

    #[test]
    fn test_custom_writer_receives_output() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).start();
        bar.tick(5);
        let out = tw.output();
        assert!(!out.is_empty(), "custom writer should receive output");
        assert!(out.contains("5/10"));
    }

    #[test]
    fn test_start_produces_initial_render() {
        let (tw, w) = make_writer();
        let _bar = ProgressBar::new(50).writer(w).start();
        let out = tw.output();
        assert!(out.contains("0/50"), "start should render initial state");
    }

    // --- Task 4.3: TTY vs non-TTY rendering ---

    #[test]
    fn test_tty_mode_uses_cr() {
        let (tw, w) = make_writer();
        let _bar = ProgressBar::new(10).writer(w).tty(true).start();
        let out = tw.output();
        assert!(
            out.starts_with('\r'),
            "TTY mode should start with \\r, got: {out}"
        );
    }

    #[test]
    fn test_non_tty_mode_uses_newlines_no_ansi() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).start(); // custom writer defaults non-TTY
        bar.tick(5);
        bar.success("done");
        let out = tw.output();
        assert!(!out.contains("\x1b["), "non-TTY should have no ANSI codes");
        assert!(out.contains('\n'), "non-TTY should use newlines");
    }

    #[test]
    fn test_custom_writer_defaults_non_tty() {
        let (tw, w) = make_writer();
        let _bar = ProgressBar::new(10).writer(w).start();
        let out = tw.output();
        // Non-TTY: no \r prefix, uses newline
        assert!(
            !out.starts_with('\r'),
            "custom writer should default to non-TTY"
        );
    }

    #[test]
    fn test_non_tty_finalization_omits_ansi() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).start();
        bar.success("all good");
        let out = tw.output();
        assert!(
            !out.contains("\x1b["),
            "non-TTY finalization should omit ANSI"
        );
        assert!(out.contains("✔"));
        assert!(out.contains("all good"));
    }

    // --- Task 4.4: Drop behavior ---

    #[test]
    fn test_drop_without_finalization_writes_newline() {
        let (tw, w) = make_writer();
        {
            let _bar = ProgressBar::new(10).writer(w).start();
            // dropped here without success/fail
        }
        let out = tw.output();
        assert!(out.ends_with('\n'), "drop should write trailing newline");
    }

    #[test]
    fn test_drop_after_finalization_no_extra_output() {
        let (tw, w) = make_writer();
        let out_before;
        {
            let bar = ProgressBar::new(10).writer(w).start();
            bar.success("done");
            out_before = tw.output();
        }
        let out_after = tw.output();
        assert_eq!(
            out_before, out_after,
            "drop after finalization should add nothing"
        );
    }

    // --- Task 4.5: Send + Sync assertion ---

    #[test]
    fn test_progress_bar_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<ProgressBar>();
    }

    #[test]
    fn test_fill_and_empty_builder_methods() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).fill('#').empty('-').start();
        bar.tick(5);
        let out = tw.output();
        assert!(out.contains('#'), "custom fill char should appear");
        assert!(out.contains('-'), "custom empty char should appear");
    }

    #[test]
    fn test_set_message_updates_output() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).start();
        bar.set_message("hello");
        bar.tick(1);
        let out = tw.output();
        assert!(out.contains("hello"), "set_message should update displayed text");
    }

    #[test]
    fn test_tty_finalize_success_has_ansi() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).tty(true).start();
        bar.success("completed");
        let out = tw.output();
        assert!(out.contains("\x1b[32m"), "TTY success should have green ANSI code");
        assert!(out.contains("✔"), "TTY success should have checkmark");
        assert!(out.contains("completed"));
    }

    #[test]
    fn test_tty_finalize_fail_has_ansi() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).tty(true).start();
        bar.fail("broken");
        let out = tw.output();
        assert!(out.contains("\x1b[31m"), "TTY fail should have red ANSI code");
        assert!(out.contains("✖"), "TTY fail should have cross mark");
        assert!(out.contains("broken"));
    }

    #[test]
    fn test_double_finalization_is_noop() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(10).writer(w).tty(true).start();
        bar.success("first");
        let out_after_first = tw.output();
        bar.fail("second");
        let out_after_second = tw.output();
        assert_eq!(out_after_first, out_after_second, "double finalization should be a no-op");
    }

    #[test]
    fn test_total_zero_normalized_to_one() {
        let (tw, w) = make_writer();
        let bar = ProgressBar::new(0).writer(w).start();
        let s = bar.state.lock().unwrap();
        assert_eq!(s.total, 1, "total of 0 should be normalized to 1");
    }


    // --- Property tests using quickcheck! macro ---

    use quickcheck::quickcheck;

    quickcheck! {
        // --- Task 4.6: Property 1 - Builder defaults ---
        // Feature: nanoprogress, Property 1: Builder defaults are correct
        // **Validates: Requirements 1.1**
        fn prop_builder_defaults(total: u64) -> bool {
            let total = total.max(1);
            let (_tw, w) = make_writer();
            let bar = ProgressBar::new(total).writer(w).start();
            let s = bar.state.lock().unwrap();
            s.config.width == 40
                && s.config.fill == '█'
                && s.config.empty == '░'
                && s.current == 0
                && s.total == total
        }

        // --- Task 4.7: Property 2 - Render output correctness ---
        // Feature: nanoprogress, Property 2: Render output is correct for any progress state
        // **Validates: Requirements 1.2, 1.3, 1.5, 2.3, 2.4, 2.5, 3.1, 3.2, 3.5**
        fn prop_render_output(current: u64, total: u64, width: u8, msg_bytes: Vec<u8>) -> bool {
            let total = total.max(1);
            let current = current.min(total);
            let width = (width as usize).max(1).min(200);
            let msg: String = String::from_utf8_lossy(&msg_bytes)
                .chars()
                .filter(|c| !c.is_control())
                .take(50)
                .collect();

            let fill = '█';
            let empty_ch = '░';

            let ratio = current as f64 / total as f64;
            let filled = (ratio * width as f64).round() as usize;
            let empty_count = width - filled;
            let percent = (ratio * 100.0) as u64;

            let expected_bar: String = std::iter::repeat_n(fill, filled)
                .chain(std::iter::repeat_n(empty_ch, empty_count))
                .collect();
            let count_str = format!("{current}/{total}");
            let pct_str = format!("{percent}%");

            let (tw, w) = make_writer();
            let bar = ProgressBar::new(total)
                .writer(w)
                .width(width)
                .message(&msg)
                .start();
            if current > 0 {
                bar.tick(current);
            }
            let out = tw.output();

            out.contains(&expected_bar)
                && out.contains(&count_str)
                && out.contains(&pct_str)
                && (msg.is_empty() || out.contains(&msg))
        }

        // --- Task 4.8: Property 3 - Tick accumulation with clamping ---
        // Feature: nanoprogress, Property 3: Tick accumulation with clamping
        // **Validates: Requirements 2.1, 2.2**
        fn prop_tick_accumulation(total: u64, ticks: Vec<u64>) -> bool {
            let total = total.max(1);
            let (_, w) = make_writer();
            let bar = ProgressBar::new(total).writer(w).start();
            for t in &ticks {
                bar.tick(*t);
            }
            let s = bar.state.lock().unwrap();
            let expected = ticks.iter().fold(0u64, |acc, t| acc.saturating_add(*t)).min(total);
            s.current == expected
        }

        // --- Task 4.9: Property 4 - Finalization output ---
        // Feature: nanoprogress, Property 4: Finalization output contains correct symbol and message
        // **Validates: Requirements 4.1, 4.2**
        fn prop_finalization_output(msg_bytes: Vec<u8>, use_success: bool) -> bool {
            let msg: String = String::from_utf8_lossy(&msg_bytes)
                .chars()
                .filter(|c| !c.is_control())
                .take(50)
                .collect();

            let (tw, w) = make_writer();
            let bar = ProgressBar::new(10).writer(w).start();
            if use_success {
                bar.success(&msg);
            } else {
                bar.fail(&msg);
            }
            let out = tw.output();

            if use_success {
                out.contains("✔") && out.contains(&msg)
            } else {
                out.contains("✖") && out.contains(&msg)
            }
        }

        // --- Task 4.10: Property 5 - Finalized bar rejects ticks ---
        // Feature: nanoprogress, Property 5: Finalized bar rejects further ticks
        // **Validates: Requirements 4.3**
        fn prop_finalized_rejects_ticks(total: u64, tick_amount: u64) -> bool {
            let total = total.max(1);
            let (tw, w) = make_writer();
            let bar = ProgressBar::new(total).writer(w).start();
            bar.success("done");
            let out_before = tw.output();
            let counter_before = bar.state.lock().unwrap().current;
            bar.tick(tick_amount.max(1));
            let out_after = tw.output();
            let counter_after = bar.state.lock().unwrap().current;
            counter_before == counter_after && out_before == out_after
        }

        // --- Task 4.11: Property 6 - No ANSI in non-TTY ---
        // Feature: nanoprogress, Property 6: Non-TTY output contains no ANSI escape codes
        // **Validates: Requirements 3.4, 4.4, 5.3**
        fn prop_no_ansi_in_non_tty(total: u64, ticks: Vec<u64>, msg_bytes: Vec<u8>, finalize: bool) -> bool {
            let total = total.max(1);
            let msg: String = String::from_utf8_lossy(&msg_bytes)
                .chars()
                .filter(|c| !c.is_control())
                .take(50)
                .collect();

            let (tw, w) = make_writer();
            let bar = ProgressBar::new(total).writer(w).message(&msg).start();
            for t in &ticks {
                bar.tick(*t);
            }
            if finalize {
                bar.success(&msg);
            } else {
                bar.fail(&msg);
            }
            let out = tw.output();
            !out.contains("\x1b[")
        }

        // --- Task 4.12: Property 7 - Concurrent tick consistency ---
        // Feature: nanoprogress, Property 7: Concurrent tick consistency
        // **Validates: Requirements 6.2**
        fn prop_concurrent_ticks(total: u64, tick_amounts: Vec<u64>) -> bool {
            let total = total.max(1);
            let tick_amounts: Vec<u64> = tick_amounts.into_iter().take(20).collect();
            let (_, w) = make_writer();
            let bar = ProgressBar::new(total).writer(w).start();

            std::thread::scope(|s| {
                for amount in &tick_amounts {
                    let bar = bar.clone();
                    let amount = *amount;
                    s.spawn(move || bar.tick(amount));
                }
            });

            let s = bar.state.lock().unwrap();
            let expected = tick_amounts.iter().fold(0u64, |acc, t| acc.saturating_add(*t)).min(total);
            s.current == expected
        }
    }
}
