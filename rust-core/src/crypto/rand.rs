use std::time::{SystemTime, UNIX_EPOCH};

pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new() -> Self {
        let start = SystemTime::now();
        let since_the_epoch = start.duration_since(UNIX_EPOCH).unwrap();
        let seed = since_the_epoch.as_nanos() as u64;
        Self { state: seed }
    }

    pub fn next(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    pub fn fill(&mut self, dest: &mut [u8]) {
        for i in 0..dest.len() {
            dest[i] = (self.next() & 0xFF) as u8;
        }
    }
}
