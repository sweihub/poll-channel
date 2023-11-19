# Rust: Poll on channel

`poll-channel` provides a way to poll on channel in Rust sync programming, crossbeam channel was used in this crate.

example
```rust
use poll_channel::{channel, Poll};

#[test]
fn poll_test() -> Result<(), crossbeam::channel::RecvError> {
    let (tx1, rx1) = channel();
    let (tx2, rx2) = channel();
    let poller = Poll::new(&[&rx1, &rx2]);

    let _ = tx1.send(100);
    let _ = tx2.send(200);
    let mut i = 0;

    while i < 3 {
        let id = poller.poll(0.01);
        if id == rx1.id() {
            let n1 = rx1.recv()?;
            assert!(n1 == 100);
            i += 1;
        } else if id == rx2.id() {
            let n2 = rx2.recv()?;
            assert!(n2 == 200);
            i += 1;
        } else if id == -1 {
            // timeout
            i += 1;
            break;
        }
    }

    Ok(())
}
```
