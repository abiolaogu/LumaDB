use serde_json::{Value, Map};
use std::collections::HashMap;

/// HashJoiner implements an in-memory Hash Join algorithm.
pub struct HashJoiner;

impl HashJoiner {
    /// Executes a hash join on two vectors of JSON objects.
    /// 
    /// # Arguments
    /// * `left` - The left (build) side of the join. Ideally smaller.
    /// * `right` - The right (probe) side of the join.
    /// * `key` - The field name to join on (MVP: assumes same key name in both).
    /// 
    /// # Returns
    /// * `Vec<Value>` - The joined rows.
    pub fn execute(left: Vec<Value>, right: Vec<Value>, key: &str) -> Vec<Value> {
        // 1. Build Phase: Create Hash Map from Left Relation
        let mut hash_table: HashMap<String, Vec<&Map<String, Value>>> = HashMap::new();

        for item in &left {
            if let Value::Object(map) = item {
                if let Some(val) = map.get(key) {
                    // Convert value to string for key (handles numbers/strings)
                    // In a real DB, we'd use typed keys.
                    let key_str = Self::val_to_string(val);
                    hash_table.entry(key_str).or_default().push(map);
                }
            }
        }

        // 2. Probe Phase: Iterate Right Relation
        let mut results = Vec::new();

        for item in right {
            if let Value::Object(right_map) = item {
                if let Some(val) = right_map.get(key) {
                    let key_str = Self::val_to_string(val);
                    
                    if let Some(matches) = hash_table.get(&key_str) {
                        for left_map in matches {
                            // Merge left and right
                            let mut merged = Map::new();
                            
                            // Prefix left keys to avoid collisions (MVP)
                            for (k, v) in *left_map {
                                merged.insert(format!("left_{}", k), v.clone());
                            }
                            
                            // Add right keys
                            for (k, v) in &right_map {
                                merged.insert(format!("right_{}", k), v.clone());
                            }
                            
                            results.push(Value::Object(merged));
                        }
                    }
                }
            }
        }

        results
    }

    fn val_to_string(v: &Value) -> String {
        match v {
            Value::String(s) => s.clone(),
            Value::Number(n) => n.to_string(),
            Value::Bool(b) => b.to_string(),
            _ => v.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_hash_join() {
        let left = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];
        let right = vec![
            json!({"user_id": 1, "action": "login"}),
            json!({"user_id": 1, "action": "logout"}),
            json!({"user_id": 3, "action": "view"}), // No match
        ];

        // MVP limitation: Join function assumes same key name currently, 
        // OR we need to pass left_key and right_key. 
        // The implementation uses `key` for both.
        // Let's adjust the test to have matching keys or update impl.
        // Updating test data to match implementation key assumption or updating impl.
        // For MVP simplicity in `execute`, let's assume we normalize keys or pass distinct ones.
        // But `execute(key)` uses one string.
        // Let's fix implementation to take left_key and right_key if needed,
        // or just ensure data aligns.
        // Let's use "id" for standard test data alignment.
        
        let right_aligned = vec![
             json!({"id": 1, "action": "login"}),
             json!({"id": 1, "action": "logout"}),
             json!({"id": 3, "action": "view"}),
        ];

        let results = HashJoiner::execute(left, right_aligned, "id");
        
        assert_eq!(results.len(), 2);
        // Alice had 2 actions. Bob had 0. User 3 had 1 but no User record.
        
        let first = &results[0]; // Alice login or logout (order depends on hashmap implementation? No, probe order is deterministic)
        // Probe order (right side) determines output order mostly, but build side collision order matters.
        // Right side order is preserved.
        
        // Check content
        let obj = first.as_object().unwrap();
        assert!(obj.contains_key("left_name"));
        assert!(obj.contains_key("right_action"));
    }
}
