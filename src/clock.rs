use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

/// A clock that can be used to update listeners on a regular interval.
pub struct Clock {
    stop_flag: Arc<AtomicBool>,
    timer_handle: Option<JoinHandle<()>>,
    pub interval: Duration,
    listeners: Vec<Sender<()>>,
}

impl Clock {
    pub fn new(interval: Duration) -> Self {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let timer_handle = None;
        let interval = interval;
        let listeners = Vec::new();
        Clock {
            stop_flag,
            timer_handle,
            interval,
            listeners,
        }
    }
    /// Starts the clock.
    ///
    /// This function starts a thread that will update any attached listeners on the specified interval.
    pub fn start(&mut self) {
        let mut last_update = Instant::now();
        let stop_flag = Arc::clone(&self.stop_flag);
        let interval = self.interval.clone();
        let listeners = self.listeners.clone();
        self.timer_handle = Some(thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let now = Instant::now();
                if now - last_update >= interval {
                    for listener in &listeners {
                        let _ = listener.send(());
                    }
                    last_update = now;
                }
            }
        }));
    }

    pub fn teardown(&mut self) -> Result<(), &str> {
        self.stop_flag.store(true, Ordering::Relaxed);
        for listener in self.listeners.drain(..) {
            drop(listener)
        }
        if let Some(handle) = self.timer_handle.take() {
            handle.join().map_err(|_| "thread panicked")
        } else {
            Ok(())
        }
    }
    /// Get a receiver node from the clock.
    ///
    /// This function should only be used before starting the clock, and this is enforced with an error.
    pub fn become_listener(&mut self) -> Result<Receiver<()>, &str> {
        if self.timer_handle.is_some() {
            Err("cannot become listener after clock has started")
        } else if !self.stop_flag.load(Ordering::Relaxed) {
            let (tx, rx) = mpsc::channel();
            self.listeners.push(tx);
            Ok(rx)
        } else {
            Err("clock has been terminated")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock() {
        let mut clock = Clock::new(Duration::from_micros(16_667));
        if let Ok(rx1) = clock.become_listener() {
            if let Ok(rx2) = clock.become_listener() {
                let t1 = thread::spawn(move || {
                    let mut count = 0;
                    while count < 61 {
                        println!("count on t1: {}", count);
                        let _ = rx1.recv();
                        count += 1;
                    }
                    count
                });
                let t2 = thread::spawn(move || {
                    let mut count = 0;
                    while count < 61 {
                        println!("count on t2: {}", count);
                        let _ = rx2.recv();
                        count += 1;
                    }
                    count
                });

                clock.start();

                let threads = vec![t1, t2];
                for thread in threads {
                    assert_eq!(thread.join().unwrap(), 61);
                }
            }
        } else {
            assert!(false, "could not register listeners")
        }
    }
    #[test]
    fn test_clock_stop() {
        let mut clock = Clock::new(Duration::from_micros(16_667));
        if let Ok(rx1) = clock.become_listener() {
            if let Ok(rx2) = clock.become_listener() {
                let t1 = thread::spawn(move || {
                    while let Ok(_) = rx1.recv() {
                        thread::sleep(Duration::from_millis(1))
                    }
                    21
                });
                let t2 = thread::spawn(move || {
                    while let Ok(_) = rx2.recv() {
                        thread::sleep(Duration::from_millis(1))
                    }
                    21
                });

                clock.start();
                thread::sleep(Duration::from_millis(500));
                assert!(
                    clock.teardown().is_ok(),
                    "timer thread is not safely joined"
                );

                let threads = vec![t1, t2];
                for thread in threads {
                    assert_eq!(thread.join().unwrap(), 21);
                }
            }
        } else {
            assert!(false, "could not register listeners")
        }
    }
}
