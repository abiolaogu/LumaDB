use std::collections::HashMap;

pub struct SystemVariables {
    vars: HashMap<String, String>,
}

impl SystemVariables {
    pub fn new() -> Self {
        let mut vars = HashMap::new();
        vars.insert("version".to_string(), "8.0.35-LumaDB".to_string());
        vars.insert("version_comment".to_string(), "LumaDB MySQL Adapter".to_string());
        vars.insert("sql_mode".to_string(), "ONLY_FULL_GROUP_BY,STRICT_TRANS_TABLES,NO_ZERO_IN_DATE,NO_ZERO_DATE,ERROR_FOR_DIVISION_BY_ZERO,NO_ENGINE_SUBSTITUTION".to_string());
        vars.insert("autocommit".to_string(), "1".to_string());
        vars.insert("character_set_client".to_string(), "utf8mb4".to_string());
        vars.insert("character_set_connection".to_string(), "utf8mb4".to_string());
        vars.insert("character_set_results".to_string(), "utf8mb4".to_string());
        vars.insert("collation_connection".to_string(), "utf8mb4_general_ci".to_string());
        vars.insert("max_allowed_packet".to_string(), "67108864".to_string());
        vars.insert("transaction_isolation".to_string(), "REPEATABLE-READ".to_string());
        Self { vars }
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.vars.get(name)
    }
}
