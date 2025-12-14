
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorQuantizer;

impl VectorQuantizer {
    pub fn quantize_f32_to_int8(vector: &[f32]) -> (Vec<i8>, f32, f32) {
        // Simple min-max quantization
        let min_val = vector.iter().fold(f32::INFINITY, |a, &b| a.min(b));
        let max_val = vector.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        
        let range = max_val - min_val;
        let scale = if range == 0.0 { 1.0 } else { 255.0 / range };
        
        let quantized = vector.iter().map(|&v| {
            let normalized = (v - min_val) * scale;
            (normalized - 128.0) as i8
        }).collect();
        
        (quantized, min_val, scale)
    }
    
    pub fn dequantize_int8_to_f32(quantized: &[i8], min_val: f32, scale: f32) -> Vec<f32> {
        quantized.iter().map(|&q| {
            let normalized = (q as f32) + 128.0;
            (normalized / scale) + min_val
        }).collect()
    }
}
