
use crate::Result;
use std::collections::HashMap;
use luma_protocol_core::Value;

#[derive(Debug, Clone)]
pub struct LineProtocolPoint {
    pub measurement: String,
    pub tags: HashMap<String, String>,
    pub fields: HashMap<String, Value>,
    pub timestamp: Option<i64>,
}

pub struct LineProtocolParser;

impl LineProtocolParser {
    pub fn parse(input: &str) -> Result<Vec<LineProtocolPoint>> {
        let mut points = Vec::new();
        
        for line in input.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Ok(point) = Self::parse_line(line) {
                points.push(point);
            }
        }
        
        Ok(points)
    }

    fn parse_line(line: &str) -> Result<LineProtocolPoint> {
        // Format: measurement,tag_set field_set timestamp
        // Escape spaces in keys/values with backslash. Simpler split for now.
        
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(luma_protocol_core::ProtocolError::Protocol("Invalid Line Protocol format".into()));
        }

        // 1. Tag Set (and Measurement)
        let tag_part = parts[0];
        let (measurement, tags) = Self::parse_tag_part(tag_part)?;

        // 2. Field Set
        let field_part = parts[1];
        let fields = Self::parse_field_part(field_part)?;

        // 3. Timestamp (Optional)
        let timestamp = if parts.len() > 2 {
            parts[2].parse::<i64>().ok()
        } else {
            None // Server should assign now()
        };

        Ok(LineProtocolPoint {
            measurement,
            tags,
            fields,
            timestamp,
        })
    }

    fn parse_tag_part(input: &str) -> Result<(String, HashMap<String, String>)> {
        // "measurement,tag1=val1,tag2=val2"
        let mut split = input.split(',');
        let measurement = split.next().ok_or_else(|| luma_protocol_core::ProtocolError::Protocol("Missing measurement".into()))?.to_string();
        
        let mut tags = HashMap::new();
        for tag_pair in split {
            if let Some((k, v)) = tag_pair.split_once('=') {
                tags.insert(k.to_string(), v.to_string());
            }
        }
        
        Ok((measurement, tags))
    }

    fn parse_field_part(input: &str) -> Result<HashMap<String, Value>> {
        // "field1=val1,field2=val2"
        // String values are quoted.
        let mut fields = HashMap::new();
        for field_pair in input.split(',') {
            if let Some((k, v)) = field_pair.split_once('=') {
                let value = if v.ends_with('i') {
                    // Integer
                    v.trim_end_matches('i').parse::<i64>()
                        .map(Value::Int64)
                        .unwrap_or(Value::Null)
                } else if v == "t" || v == "T" || v == "true" {
                    Value::Bool(true)
                } else if v == "f" || v == "F" || v == "false" {
                    Value::Bool(false)
                } else if v.starts_with('"') && v.ends_with('"') {
                    // String
                    Value::Text(v.trim_matches('"').to_string())
                } else {
                    // Float
                    v.parse::<f64>()
                        .map(Value::Float64)
                        .unwrap_or(Value::Null)
                };
                
                fields.insert(k.to_string(), value);
            }
        }
        Ok(fields)
    }
}
