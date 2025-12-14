
use std::collections::HashMap;
use zstd;

/// Global dictionary for frequently repeated values
pub struct GlobalDictionary {
    pub string_to_id: HashMap<String, u32>,
    pub id_to_string: Vec<String>,
    pub frequency: HashMap<String, u64>,
}

impl GlobalDictionary {
    pub fn new() -> Self {
        Self {
            string_to_id: HashMap::new(),
            id_to_string: Vec::new(),
            frequency: HashMap::new(),
        }
    }
    
    pub fn encode(&mut self, value: &str) -> u32 {
        if let Some(&id) = self.string_to_id.get(value) {
            *self.frequency.get_mut(value).unwrap() += 1;
            return id;
        }
        let id = self.id_to_string.len() as u32;
        self.string_to_id.insert(value.to_string(), id);
        self.id_to_string.push(value.to_string());
        self.frequency.insert(value.to_string(), 1);
        id
    }
    
    pub fn decode(&self, id: u32) -> &str {
        &self.id_to_string[id as usize]
    }
}

pub struct TimestampCompressor {
    base: i64,
    prev: i64,
    prev_delta: i64,
    deltas: Vec<i64>,
}

impl TimestampCompressor {
    pub fn new() -> Self {
        Self { base: 0, prev: 0, prev_delta: 0, deltas: Vec::new() }
    }
    
    pub fn compress(&mut self, timestamp: i64) {
        if self.base == 0 {
            self.base = timestamp;
            self.prev = timestamp;
            return;
        }
        let delta = timestamp - self.prev;
        if self.prev_delta == 0 {
            self.deltas.push(delta);
            self.prev_delta = delta;
            self.prev = timestamp;
            return;
        }
        let delta_of_delta = delta - self.prev_delta;
        self.deltas.push(delta_of_delta);
        self.prev_delta = delta;
        self.prev = timestamp;
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.base.to_le_bytes());
        if !self.deltas.is_empty() {
            bytes.extend_from_slice(&self.deltas[0].to_le_bytes());
        }
        for &delta in &self.deltas[1..] {
             bytes.extend_from_slice(&encode_varint(delta));
        }
        zstd::encode_all(&bytes[..], 3).unwrap()
    }
}

fn encode_varint(mut value: i64) -> Vec<u8> {
    let mut bytes = Vec::new();
    loop {
        let mut byte = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 { byte |= 0x80; }
        bytes.push(byte);
        if value == 0 { break; }
    }
    bytes
}

// LogEntryCompressor would combine these
// For brevity matching the user request, implementing core components first.
