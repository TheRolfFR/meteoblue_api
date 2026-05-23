use std::io;
use std::io::prelude::*;

pub(crate) fn pause() {
    // If not running in headless mode, pause execution until browser is closed manually
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    // We want the cursor to stay at the end of the line, so we print without a newline and flush manually.
    writeln!(stdout, "Press any key to continue...").expect("Shall press key");
    stdout.flush().expect("Failed to flush");

    // Read a single byte and discard
    let _ = stdin.read(&mut [0u8]).expect("Failed to read byte for pause");
}
