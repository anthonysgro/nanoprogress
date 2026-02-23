use nanoprogress::ProgressBar;
use std::thread;
use std::time::Duration;

fn main() {
    let bar = ProgressBar::new(100)
        .width(25)
        .message("Downloading...")
        .start();

    for _ in 0..100 {
        thread::sleep(Duration::from_millis(30));
        bar.tick(1);
    }
    bar.success("Download complete");

    let bar = ProgressBar::new(50)
        .width(25)
        .fill('#')
        .empty('-')
        .message("Installing...")
        .start();

    for _ in 0..50 {
        thread::sleep(Duration::from_millis(40));
        bar.tick(1);
    }
    bar.success("Installed");

    let bar = ProgressBar::new(20)
        .width(25)
        .message("Compiling...")
        .start();

    for i in 0..20 {
        thread::sleep(Duration::from_millis(50));
        bar.tick(1);
        if i == 12 {
            bar.fail("Build failed at step 13");
            break;
        }
    }
}
