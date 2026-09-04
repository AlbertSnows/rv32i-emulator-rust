use std::io::Read;
use std::sync::mpsc;

pub fn read_stdin_loop(tx: mpsc::Sender<u8>) {
    let stdin = std::io::stdin();
    for byte in stdin.lock().bytes() {
        match byte {
            Ok(b) => {
                // tx.send(b) pushes a byte to the channel, returns Result
                // if failure, stop iterating over loop, thread ends
                // only fails when rx fails
                // rx only fails when main loop stops running
                if tx.send(b).is_err() { break; }
            },
            Err(_) => break,
        }
    }
}