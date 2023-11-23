use poll_channel::{channel, Poll};

#[test]
fn poll_test() -> Result<(), crossbeam::channel::RecvError> {
    let (tx1, rx1) = channel();
    let (tx2, rx2) = channel();

    let poller = Poll::new();
    poller.append(&[&rx1, &rx2]);

    // thread
    let tx1_clone = tx1.clone();
    let bg = std::thread::spawn(move || {
        let _ = tx1_clone.send(1000);
    });
    let _ = bg.join();

    let _ = tx1.send(100);
    let _ = tx2.send(200);
    let mut i = 0;

    while i < 4 {
        let token = poller.poll(0.01);
        if token == rx1.token() {
            let n1 = rx1.recv()?;
            assert!(n1 == 100 || n1 == 1000);
            i += 1;
        } else if token == rx2.token() {
            let n2 = rx2.recv()?;
            assert!(n2 == 200);
            i += 1;
        } else if token == -1 {
            // timeout
            i += 1;
            break;
        }
    }

    assert!(i == 4);

    Ok(())
}
