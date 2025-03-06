const RAM_SIZE: usize = 4096;
const REGISTER_COUNT: usize = 16;
const STACK_SIZE: u8 = 16;
const RUNLOOP_TIMER_DEFAULT: usize = 8;
const PROGRAM_START: usize = 0x200;

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
    pub fn push(&mut self, value: u16) -> Result<(), u8> {
        if self.p >= STACK_SIZE {
            Err(self.p)
        } else {
            self.memory[self.p as usize] = value;
            self.p += 1;
            Ok(())
        }
    }
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
}
