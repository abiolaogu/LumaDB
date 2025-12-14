
pub struct SimdAggregates;

impl SimdAggregates {
    /// Sum array of f64 using SIMD if available, else scalar
    pub fn sum_f64(values: &[f64]) -> f64 {
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            unsafe { Self::sum_f64_avx2(values) }
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
        {
            values.iter().sum()
        }
    }

    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn sum_f64_avx2(values: &[f64]) -> f64 {
        use std::arch::x86_64::*;
        let mut sum = _mm256_setzero_pd();
        let chunks = values.chunks_exact(4);
        let remainder = chunks.remainder();
        
        for chunk in chunks {
            let v = _mm256_loadu_pd(chunk.as_ptr());
            sum = _mm256_add_pd(sum, v);
        }
        
        let sum_128 = _mm_add_pd(
            _mm256_castpd256_pd128(sum),
            _mm256_extractf128_pd(sum, 1),
        );
        let sum_64 = _mm_add_pd(sum_128, _mm_shuffle_pd(sum_128, sum_128, 1));
        
        let mut result = _mm_cvtsd_f64(sum_64);
        
        for &v in remainder {
            result += v;
        }
        
        result
    }

    pub fn filter_gt_f64(values: &[f64], threshold: f64) -> Vec<u64> {
         #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
             let mut output = Vec::with_capacity(values.len());
             unsafe { Self::filter_gt_f64_avx2(values, threshold, &mut output); }
             output
        }
        #[cfg(not(all(target_arch = "x86_64", target_feature = "avx2")))]
        {
            values.iter()
                .enumerate()
                .filter(|(_, &v)| v > threshold)
                .map(|(i, _)| i as u64)
                .collect()
        }
    }
    
    #[cfg(target_arch = "x86_64")]
    #[target_feature(enable = "avx2")]
    pub unsafe fn filter_gt_f64_avx2(
        values: &[f64],
        threshold: f64,
        output: &mut Vec<u64>,
    ) {
        use std::arch::x86_64::*;
        let threshold_vec = _mm256_set1_pd(threshold);
        
        for (i, chunk) in values.chunks_exact(4).enumerate() {
            let v = _mm256_loadu_pd(chunk.as_ptr());
            let cmp = _mm256_cmp_pd(v, threshold_vec, _CMP_GT_OQ);
            let mask = _mm256_movemask_pd(cmp);
            
            for bit in 0..4 {
                if (mask & (1 << bit)) != 0 {
                    output.push((i * 4 + bit) as u64);
                }
            }
        }
        // Handle remainder logic if needed (omitted for brevity in prompt loop, but needed for correctness)
        let remainder_start = values.len() - values.len() % 4;
         for i in remainder_start..values.len() {
             if values[i] > threshold {
                 output.push(i as u64);
             }
         }
    }
}
