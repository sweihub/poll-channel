//!# Rust: Poll on channel
//!
//!`poll-channel` provides a way to poll on channel in Rust sync programming, crossbeam channel was used in this crate.
//!
//!example
//!```rust
//!use poll_channel::{channel, Poll};
//!
//!#[test]
//!fn poll_test() -> Result<(), crossbeam::channel::RecvError> {
//!    let (tx1, rx1) = channel();
//!    let (tx2, rx2) = channel();
//!
//!    let poller = Poll::new();
//!    poller.append([&rx1, &rx2]);
//!
//!    let _ = tx1.send(100);
//!    let _ = tx2.send(200);
//!    let mut i = 0;
//!
//!    while i < 3 {
//!        let tag = poller.poll(0.01);
//!        if tag == rx1.tag() {
//!            let n1 = rx1.recv()?;
//!            assert!(n1 == 100);
//!            i += 1;
//!        } else if tag == rx2.tag() {
//!            let n2 = rx2.recv()?;
//!            assert!(n2 == 200);
//!            i += 1;
//!        } else if tag == -1 {
//!            // timeout
//!            i += 1;
//!            break;
//!        }
//!    }
//!
//!    Ok(())
//!}
//!```
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

pub use crossbeam::channel::RecvError;
pub use crossbeam::channel::RecvTimeoutError;
pub use crossbeam::channel::SendError;
pub use crossbeam::channel::TryRecvError;

pub struct Signal {
    tx: crossbeam::channel::Sender<i32>,
    rx: crossbeam::channel::Receiver<i32>,
}

impl Signal {
    fn new() -> Self {
        let (tx, rx) = crossbeam::channel::unbounded();
        Self { tx, rx }
    }
}

pub struct Sender<T> {
    init: Mutex<bool>,
    producer: Mutex<Option<SignalSender>>,
    signal: ArcMutex2<OptionSignal>,
    tx: crossbeam::channel::Sender<T>,
    tag: i32,
}

pub type SignalSender = crossbeam::channel::Sender<i32>;
pub type OptionSignal = Option<Signal>;
pub type ArcMutex<T> = Arc<Mutex<T>>;
pub type ArcMutex2<T> = ArcMutex<ArcMutex<T>>;
static TAG: Mutex<i32> = Mutex::new(0);

pub struct Receiver<T> {
    signal: ArcMutex2<OptionSignal>,
    rx: crossbeam::channel::Receiver<T>,
    tag: i32,
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Mutex::new(None));
    let signal = Arc::new(Mutex::new(inner));
    let (tx, rx) = crossbeam::channel::unbounded();
    let mut id = TAG.lock().unwrap();
    let next = *id;
    *id += 1;
    let receiver = Receiver {
        signal,
        rx,
        tag: next,
    };
    let sender = Sender {
        producer: Mutex::new(None),
        signal: receiver.signal.clone(),
        tx,
        tag: next,
        init: Mutex::new(false),
    };
    (sender, receiver)
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Self {
            init: Mutex::new(false),
            producer: Mutex::new(None),
            signal: self.signal.clone(),
            tx: self.tx.clone(),
            tag: self.tag,
        }
    }
}

impl<T> Sender<T> {
    pub fn send(&self, data: T) -> Result<(), SendError<T>> {
        // avoid mutable, no one races for the mutexes
        let mut init = self.init.lock().unwrap();
        let mut producer = self.producer.lock().unwrap();
        if !*init {
            *init = true;
            let inner = self.signal.lock().unwrap();
            let signal = inner.lock().unwrap();
            if signal.is_some() {
                let tx = signal.as_ref().unwrap().tx.clone();
                *producer = Some(tx);
            }
        }
        let result = self.tx.send(data);
        if let Some(signal) = &*producer {
            let _ = signal.send(self.tag);
        }
        return result;
    }
}

impl<T> Receiver<T> {
    /// channel id
    pub fn tag(&self) -> i32 {
        self.tag
    }

    pub fn recv(&self) -> Result<T, RecvError> {
        self.rx.recv()
    }

    pub fn recv_timeout(
        &self,
        timeout: Duration,
    ) -> Result<T, crossbeam::channel::RecvTimeoutError> {
        self.rx.recv_timeout(timeout)
    }

    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        self.rx.try_recv()
    }

    pub fn len(&self) -> usize {
        self.rx.len()
    }
}

pub trait Pollable {
    /// shared signal channel
    fn signal(&self) -> ArcMutex2<OptionSignal>;
    /// channel id
    fn tag(&self) -> i32;
}

impl<T> Pollable for Receiver<T> {
    fn signal(&self) -> ArcMutex2<OptionSignal> {
        self.signal.clone()
    }

    fn tag(&self) -> i32 {
        self.tag
    }
}

pub struct Poll {
    signal: ArcMutex<OptionSignal>,
}

impl Poll {
    pub fn new() -> Self {
        let instance = Signal::new();
        let inner = Arc::new(Mutex::new(Some(instance)));
        Self { signal: inner }
    }

    /// Append list of receivers
    pub fn append<T: Pollable>(&self, receivers: &[&T]) {
        for i in receivers {
            self.add(*i);
        }
    }

    /// Add single receiver
    pub fn add<T: Pollable>(&self, receiver: &T) {
        let outer = receiver.signal();
        let mut inner = outer.lock().unwrap();
        *inner = self.signal.clone();
    }

    /// Poll with decimal seconds timeout, return channel id, -1 for timeout.
    pub fn poll(&self, timeout: f32) -> i32 {
        let timeout = Duration::from_nanos((timeout * 1e9) as u64);
        // single reader
        let signal = self.signal.lock().unwrap();
        signal
            .as_ref()
            .unwrap()
            .rx
            .recv_timeout(timeout)
            .unwrap_or(-1)
    }
}
