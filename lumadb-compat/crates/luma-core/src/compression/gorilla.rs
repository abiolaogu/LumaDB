
/// XOR Compression for Floating Point Values (Gorilla)
/// Uses proper bitpacking for 8x memory efficiency
pub struct GorillaEncoder {
    last_value: u64,
    leading_zeros: u8,
    trailing_zeros: u8,
    buffer: Vec<u8>,      // Packed bytes
    bit_position: u8,     // Current bit position in last byte (0-7)
}

impl GorillaEncoder {
    pub fn new() -> Self {
        Self {
            last_value: 0,
            leading_zeros: 64,
            trailing_zeros: 64,
            buffer: Vec::new(),
            bit_position: 0,
        }
    }

    pub fn encode(&mut self, value: f64) {
        let value_bits = value.to_bits();
        let xor = value_bits ^ self.last_value;

        if xor == 0 {
            // Write single 0 bit
            self.write_bit(false);
        } else {
            // Write 1 bit
            self.write_bit(true);
            
            let leading = xor.leading_zeros() as u8;
            let trailing = xor.trailing_zeros() as u8;

            if leading >= self.leading_zeros && trailing >= self.trailing_zeros {
                // Control bit = 0: reuse previous block size
                self.write_bit(false);
                // Write meaningful bits using previous leading/trailing
                let significant_bits = 64 - self.leading_zeros - self.trailing_zeros;
                let shifted = xor >> self.trailing_zeros;
                self.write_bits(shifted, significant_bits);
            } else {
                // Control bit = 1: new block size
                self.write_bit(true);
                // Write 5 bits for leading zeros count
                self.write_bits(leading as u64, 5);
                // Write 6 bits for significant bits length
                let significant_bits = 64 - leading - trailing;
                self.write_bits(significant_bits as u64, 6);
                // Write the significant bits
                let shifted = xor >> trailing;
                self.write_bits(shifted, significant_bits);
                
                self.leading_zeros = leading;
                self.trailing_zeros = trailing;
            }
        }
        self.last_value = value_bits;
    }

    /// Write a single bit to the buffer
    fn write_bit(&mut self, bit: bool) {
        if self.bit_position == 0 {
            self.buffer.push(0);
        }
        if bit {
            let last_idx = self.buffer.len() - 1;
            self.buffer[last_idx] |= 1 << (7 - self.bit_position);
        }
        self.bit_position = (self.bit_position + 1) % 8;
    }

    /// Write multiple bits (up to 64) to the buffer
    fn write_bits(&mut self, value: u64, count: u8) {
        for i in (0..count).rev() {
            self.write_bit((value >> i) & 1 == 1);
        }
    }

    /// Get the compressed data
    pub fn finish(self) -> Vec<u8> {
        self.buffer
    }

    /// Get current compression ratio
    pub fn compression_ratio(&self, original_count: usize) -> f64 {
        let original_bytes = original_count * 8; // f64 = 8 bytes
        let compressed_bytes = self.buffer.len();
        if compressed_bytes == 0 {
            0.0
        } else {
            original_bytes as f64 / compressed_bytes as f64
        }
    }
}

/// Delta-of-Delta Compression for Timestamps
pub struct DeltaDeltaEncoder {
    last_val: i64,
    last_delta: i64,
    buffer: Vec<i64>, // Store deltas directly for SIMD/integer compression later
}

impl DeltaDeltaEncoder {
    pub fn new() -> Self {
        Self {
            last_val: 0,
            last_delta: 0,
            buffer: Vec::new(),
        }
    }

    pub fn encode(&mut self, timestamp: i64) {
        if self.last_val == 0 {
            self.last_val = timestamp;
            self.buffer.push(timestamp);
            return;
        }

        let delta = timestamp - self.last_val;
        let delta_of_delta = delta - self.last_delta;
        
        // In real Gorilla, we'd varint encode this DOD
        self.buffer.push(delta_of_delta);
        
        self.last_delta = delta;
        self.last_val = timestamp;
    }
}
