use std::{
    sync::{
        atomic::{AtomicBool, AtomicU8, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

// TODO: most of these should be configurable
const RAM_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
const STACK_SIZE: u8 = 16;
const RUNLOOP_TIMER_DEFAULT: u8 = 8;
const PROGRAM_START: usize = 0x200;
const TIMER_INTERVAL: Duration = Duration::from_micros(16_667);

/// A stack component built on top of a fixed-size array with Result<> types to prevent overflows and underflows.
#[derive(Debug)]
pub struct Stack {
    memory: [u16; STACK_SIZE as usize],
    p: u8,
}

impl Stack {
    pub fn new() -> Stack {
        Stack {
            memory: [0; STACK_SIZE as usize],
            p: 0,
        }
    }
    /// Pushes a value onto the stack. On error, it will give you the stack pointer.
    pub fn push(&mut self, value: u16) -> Result<(), u8> {
        if self.p >= STACK_SIZE {
            Err(self.p)
        } else {
            self.memory[self.p as usize] = value;
            self.p += 1;
            Ok(())
        }
    }
    /// Pops a value from the stack, giving you the value if successful, or the stack pointer if unsuccessful.
    pub fn pop(&mut self) -> Result<u16, u8> {
        if self.p == 0 {
            Err(0)
        } else if self.p >= STACK_SIZE {
            Err(self.p)
        } else {
            self.p -= 1;
            Ok(self.memory[self.p as usize])
        }
    }
}

/// A timer component that meets Chip8 specifications, and a thread to guarantee a 60hz clock cycle.
/// Must be `start()`ed before use, and must be `teardown()`ed after use.
pub struct Timers {
    delay_timer: Arc<AtomicU8>,
    sound_timer: Arc<AtomicU8>,
    timer_handle: Option<JoinHandle<()>>,
    stop_flag: Arc<AtomicBool>,
}

impl Timers {
    pub fn new() -> Timers {
        Timers {
            delay_timer: Arc::new(AtomicU8::new(RUNLOOP_TIMER_DEFAULT)),
            sound_timer: Arc::new(AtomicU8::new(RUNLOOP_TIMER_DEFAULT)),
            timer_handle: None,
            stop_flag: Arc::new(AtomicBool::new(false)),
        }
    }
    /// Starts the timer.
    ///
    /// This function starts a thread that will update every 1/60th of a second, subtracting one from
    /// nonzero values of the delay and sound timers. The thread can be terminated with `teardown()`.
    pub fn start(&mut self) {
        let mut last_update = Instant::now();
        let delay_timer = Arc::clone(&self.delay_timer);
        let sound_timer = Arc::clone(&self.sound_timer);
        let stop_flag = Arc::clone(&self.stop_flag);
        self.timer_handle = Some(thread::spawn(move || {
            while !stop_flag.load(Ordering::Relaxed) {
                let now = Instant::now();
                if now - last_update >= TIMER_INTERVAL {
                    if delay_timer.load(Ordering::SeqCst) > 0 {
                        delay_timer.fetch_sub(1, Ordering::SeqCst);
                    }
                    if sound_timer.load(Ordering::SeqCst) > 0 {
                        sound_timer.fetch_sub(1, Ordering::SeqCst);
                    }
                    last_update = now;
                }
            }
        }));
    }

    /// Stops the timer.
    pub fn teardown(&mut self) -> Result<(), &str> {
        self.stop_flag.store(true, Ordering::Relaxed);
        if self.timer_handle.is_some() {
            self.timer_handle
                .take()
                .unwrap()
                .join()
                .map_err(|_| "thread panicked")
        } else {
            Ok(())
        }
    }

    pub fn retrieve_delay_timer(&self) -> u8 {
        self.delay_timer.load(Ordering::SeqCst)
    }

    pub fn set_delay_timer(&self, value: u8) {
        self.delay_timer.store(value, Ordering::SeqCst);
    }

    pub fn set_sound_timer(&self, value: u8) {
        self.sound_timer.store(value, Ordering::SeqCst);
    }
}

pub struct CPU {
    ram: [u8; RAM_SIZE],
    registers: [u8; REGISTER_COUNT],
    stack: Stack,
    pc: u16,
    index: u16,
    delay_timer: u8,
    sound_timer: u8,
    //display: Display,
}

impl CPU {
    pub fn new() -> CPU {
        CPU {
            ram: [0; RAM_SIZE],
            registers: [0; REGISTER_COUNT],
            stack: Stack::new(),
            pc: 0,
            index: 0,
            delay_timer: 0,
            sound_timer: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_push_pop() {
        let mut stack = Stack::new();
        assert!(stack.push(0xEF).is_ok(), "push failed on 0xEF");
        assert!(stack.push(0xAB).is_ok(), "push failed on 0xAB");
        assert_eq!(stack.pop(), Ok(0xAB), "stack is not last-in first-out");
        assert_eq!(stack.pop(), Ok(0xEF), "stack is not last-in first-out");
    }
    #[test]
    fn pop_on_empty() {
        let mut stack = Stack::new();
        assert!(stack.pop().is_err());
    }
    #[test]
    fn stack_overflow() {
        let mut stack = Stack::new();
        for i in 1..17 {
            assert!(stack.push(i as u16).is_ok());
        }
        assert!(stack.push(0xEF).is_err());
    }

    #[test]
    fn timer_works() {
        let mut timers = Timers::new();
        timers.set_delay_timer(30);
        timers.set_sound_timer(240);
        timers.start();
        thread::sleep(Duration::from_millis(500));
        assert_eq!(
            timers.retrieve_delay_timer(),
            0,
            "timer does not count down as expected"
        );
        assert!(
            timers.teardown().is_ok(),
            "timer thread is not safely joined"
        );
    }
}
