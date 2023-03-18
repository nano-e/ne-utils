pub struct CircularBuffer {
    buffer: Vec<u8>,
    capacity: usize,
    start: usize,
    end: usize,
}

impl CircularBuffer {
    pub fn new(capacity: usize) -> Self {
        CircularBuffer {
            buffer: vec![0; capacity],
            capacity,
            start: 0,
            end: 0,
        }
    }

    pub fn extend(&mut self, data: &[u8]) {
        for &byte in data {
            self.push_back(byte);
        }
    }
    pub fn push_back(&mut self, value: u8) {
        self.buffer[self.end] = value;
        self.end = (self.end + 1) % self.capacity;
        
        if self.end == self.start {
            self.start = (self.start + 1) % self.capacity;
        }
    }

    pub fn pop_front(&mut self) -> Option<u8> {
        if self.start == self.end {
            None
        } else {
            let value = self.buffer[self.start];
            self.start = (self.start + 1) % self.capacity;
            Some(value)
        }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}
