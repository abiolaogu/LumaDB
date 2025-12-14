
use std::sync::{Arc, RwLock};
use std::time::Duration;
use crate::stream::view::{MaterializedView, WindowSpec, Row};
use crate::Value;

pub struct WindowedAggregation {
    pub window: WindowSpec,
    pub aggregation: Arc<MaterializedView>, // Shared view
    pub watermark: Arc<RwLock<i64>>,
    pub allowed_lateness: Duration,
}

impl WindowedAggregation {
    pub fn process_event(&self, row: &Row, event_time: i64) {
        let mut watermark = self.watermark.write().unwrap();
        *watermark = (*watermark).max(event_time);
        
        let current_wm = *watermark;
        drop(watermark); // Release lock

        if event_time < current_wm - self.allowed_lateness.as_micros() as i64 {
            // Late event
            return;
        }

        let (start, end) = self.assign_to_window(event_time);
        
        let mut windowed_row = row.clone();
        windowed_row.insert("_window_start".to_string(), Value::Int64(start));
        windowed_row.insert("_window_end".to_string(), Value::Int64(end));

        self.aggregation.on_insert(&[windowed_row]);
    }

    fn assign_to_window(&self, event_time: i64) -> (i64, i64) {
        match &self.window {
            WindowSpec::Tumbling { size } => {
                let size_micros = size.as_micros() as i64;
                let start = (event_time / size_micros) * size_micros;
                (start, start + size_micros)
            }
            WindowSpec::Sliding { size, slide } => {
                let size_micros = size.as_micros() as i64;
                let slide_micros = slide.as_micros() as i64;
                let start = (event_time / slide_micros) * slide_micros;
                (start, start + size_micros)
            }
            WindowSpec::Session { gap } => {
                let gap_micros = gap.as_micros() as i64;
                (event_time, event_time + gap_micros)
            }
        }
    }
}
