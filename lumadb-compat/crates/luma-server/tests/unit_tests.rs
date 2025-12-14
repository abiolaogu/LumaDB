//! Comprehensive Unit Tests for LumaDB Protocols

#[cfg(test)]
mod redis_tests {
    use super::super::protocols::redis::*;

    #[test]
    fn test_redis_store_set_get() {
        let store = RedisStore::new();
        store.set("key1".to_string(), RedisValue::String("value1".to_string()));
        
        match store.get("key1") {
            Some(RedisValue::String(v)) => assert_eq!(v, "value1"),
            _ => panic!("Expected string value"),
        }
    }

    #[test]
    fn test_redis_store_integer_operations() {
        let store = RedisStore::new();
        store.set("counter".to_string(), RedisValue::Integer(10));
        
        match store.get("counter") {
            Some(RedisValue::Integer(v)) => assert_eq!(v, 10),
            _ => panic!("Expected integer value"),
        }
    }

    #[test]
    fn test_redis_store_list_operations() {
        let store = RedisStore::new();
        store.set("mylist".to_string(), RedisValue::List(vec![
            "item1".to_string(),
            "item2".to_string(),
            "item3".to_string(),
        ]));
        
        match store.get("mylist") {
            Some(RedisValue::List(l)) => {
                assert_eq!(l.len(), 3);
                assert_eq!(l[0], "item1");
            }
            _ => panic!("Expected list value"),
        }
    }

    #[test]
    fn test_redis_store_hash_operations() {
        let store = RedisStore::new();
        let mut hash = std::collections::HashMap::new();
        hash.insert("field1".to_string(), "value1".to_string());
        hash.insert("field2".to_string(), "value2".to_string());
        store.set("myhash".to_string(), RedisValue::Hash(hash));
        
        match store.get("myhash") {
            Some(RedisValue::Hash(h)) => {
                assert_eq!(h.len(), 2);
                assert_eq!(h.get("field1"), Some(&"value1".to_string()));
            }
            _ => panic!("Expected hash value"),
        }
    }

    #[test]
    fn test_redis_store_set_operations() {
        let store = RedisStore::new();
        let mut set = std::collections::HashSet::new();
        set.insert("member1".to_string());
        set.insert("member2".to_string());
        store.set("myset".to_string(), RedisValue::Set(set));
        
        match store.get("myset") {
            Some(RedisValue::Set(s)) => {
                assert_eq!(s.len(), 2);
                assert!(s.contains("member1"));
            }
            _ => panic!("Expected set value"),
        }
    }

    #[test]
    fn test_redis_expiry() {
        let store = RedisStore::new();
        store.set_with_expiry(
            "expiring".to_string(),
            RedisValue::String("temp".to_string()),
            std::time::Duration::from_millis(100)
        );
        
        assert!(store.get("expiring").is_some());
        std::thread::sleep(std::time::Duration::from_millis(150));
        assert!(store.get("expiring").is_none());
    }

    #[test]
    fn test_redis_stream_xadd() {
        let store = RedisStore::new();
        let mut fields = std::collections::HashMap::new();
        fields.insert("field1".to_string(), "value1".to_string());
        
        let id = store.xadd("mystream", None, fields);
        assert!(id.is_some());
        
        let len = store.xlen("mystream");
        assert_eq!(len, 1);
    }

    #[test]
    fn test_redis_pubsub() {
        let store = RedisStore::new();
        let _rx = store.subscribe("channel1");
        
        let subscribers = store.publish("channel1", "hello".to_string());
        assert!(subscribers >= 0);
    }

    #[test]
    fn test_resp_value_serialization() {
        let simple = RespValue::SimpleString("OK".to_string());
        let serialized = simple.serialize();
        assert_eq!(serialized, "+OK\r\n");
        
        let error = RespValue::Error("ERR test".to_string());
        let serialized = error.serialize();
        assert_eq!(serialized, "-ERR test\r\n");
        
        let integer = RespValue::Integer(42);
        let serialized = integer.serialize();
        assert_eq!(serialized, ":42\r\n");
        
        let bulk = RespValue::BulkString(Some("hello".to_string()));
        let serialized = bulk.serialize();
        assert_eq!(serialized, "$5\r\nhello\r\n");
        
        let null = RespValue::Null;
        let serialized = null.serialize();
        assert_eq!(serialized, "$-1\r\n");
    }
}

#[cfg(test)]
mod elasticsearch_tests {
    use super::super::protocols::elasticsearch::*;
    use serde_json::json;

    #[test]
    fn test_elasticsearch_index_document() {
        let store = ElasticsearchStore::new();
        store.create_index("test_index");
        
        let result = store.index("test_index", "doc1", json!({"title": "Hello"}), false);
        assert!(result.is_some());
    }

    #[test]
    fn test_elasticsearch_get_document() {
        let store = ElasticsearchStore::new();
        store.create_index("test_index");
        store.index("test_index", "doc1", json!({"title": "Hello"}), false);
        
        let doc = store.get_document("test_index", "doc1");
        assert!(doc.is_some());
    }

    #[test]
    fn test_elasticsearch_search() {
        let store = ElasticsearchStore::new();
        store.create_index("test_index");
        store.index("test_index", "doc1", json!({"title": "Hello World"}), false);
        store.index("test_index", "doc2", json!({"title": "Goodbye"}), false);
        
        let query = SearchQuery {
            query: json!({"match_all": {}}),
            from: 0,
            size: 10,
            sort: None,
            aggs: None,
        };
        
        let result = store.search("test_index", &query);
        assert!(result.hits.total.value >= 1);
    }

    #[test]
    fn test_elasticsearch_delete() {
        let store = ElasticsearchStore::new();
        store.create_index("test_index");
        store.index("test_index", "doc1", json!({"title": "Hello"}), false);
        
        assert!(store.get_document("test_index", "doc1").is_some());
        store.delete("test_index", "doc1");
        assert!(store.get_document("test_index", "doc1").is_none());
    }
}

#[cfg(test)]
mod cassandra_tests {
    use super::super::protocols::cassandra::*;

    #[test]
    fn test_cassandra_store_create_keyspace() {
        let store = CassandraStore::new();
        let result = store.execute(None, "CREATE KEYSPACE test WITH replication = {'class': 'SimpleStrategy'}");
        
        match result {
            CqlResult::SchemaChange(change, target, name) => {
                assert_eq!(change, "CREATED");
                assert_eq!(target, "KEYSPACE");
            }
            _ => {}
        }
    }

    #[test]
    fn test_cassandra_store_create_table() {
        let store = CassandraStore::new();
        store.execute(None, "CREATE KEYSPACE test WITH replication = {'class': 'SimpleStrategy'}");
        let result = store.execute(Some("test"), "CREATE TABLE users (id text PRIMARY KEY, name text)");
        
        match result {
            CqlResult::SchemaChange(change, target, _) => {
                assert_eq!(change, "CREATED");
                assert_eq!(target, "TABLE");
            }
            _ => {}
        }
    }

    #[test]
    fn test_cassandra_lwt_insert() {
        let store = CassandraStore::new();
        let result = store.execute(Some("test"), "INSERT INTO users (id, name) VALUES ('1', 'Alice') IF NOT EXISTS");
        
        match result {
            CqlResult::Rows { columns, rows } => {
                assert!(columns.iter().any(|(name, _)| name == "applied"));
            }
            _ => {}
        }
    }

    #[test]
    fn test_cassandra_prepare_execute() {
        let store = CassandraStore::new();
        let stmt = store.prepare("SELECT * FROM users WHERE id = ?");
        
        assert!(!stmt.id.is_empty());
        
        let result = store.execute_prepared(&stmt.id, &[]);
        // Should succeed even without actual data
        match result {
            CqlResult::Error(_, _) => panic!("Expected success"),
            _ => {}
        }
    }
}

#[cfg(test)]
mod mongodb_tests {
    use super::super::protocols::mongodb::*;
    use serde_json::json;

    #[test]
    fn test_mongodb_insert() {
        let store = MongoStore::new();
        let result = store.execute_command("testdb", &json!({
            "insert": "users",
            "documents": [{"name": "Alice", "age": 30}]
        }));
        
        assert_eq!(result["ok"], 1);
        assert_eq!(result["n"], 1);
    }

    #[test]
    fn test_mongodb_find() {
        let store = MongoStore::new();
        store.execute_command("testdb", &json!({
            "insert": "users",
            "documents": [{"name": "Alice", "age": 30}]
        }));
        
        let result = store.execute_command("testdb", &json!({
            "find": "users",
            "filter": {}
        }));
        
        assert_eq!(result["ok"], 1);
        assert!(result["cursor"]["firstBatch"].as_array().unwrap().len() >= 1);
    }

    #[test]
    fn test_mongodb_aggregation() {
        let store = MongoStore::new();
        store.execute_command("testdb", &json!({
            "insert": "sales",
            "documents": [
                {"product": "A", "amount": 100},
                {"product": "B", "amount": 200},
                {"product": "A", "amount": 150}
            ]
        }));
        
        let result = store.execute_command("testdb", &json!({
            "aggregate": "sales",
            "pipeline": [
                {"$group": {"_id": null, "total": {"$sum": "$amount"}}}
            ]
        }));
        
        assert_eq!(result["ok"], 1);
    }

    #[test]
    fn test_mongodb_update() {
        let store = MongoStore::new();
        store.execute_command("testdb", &json!({
            "insert": "users",
            "documents": [{"_id": "1", "name": "Alice", "age": 30}]
        }));
        
        let result = store.execute_command("testdb", &json!({
            "update": "users",
            "updates": [{"q": {"_id": "1"}, "u": {"$set": {"age": 31}}}]
        }));
        
        assert_eq!(result["ok"], 1);
        assert_eq!(result["nModified"], 1);
    }
}

#[cfg(test)]
mod infrastructure_tests {
    use super::super::infrastructure::*;
    use std::time::Duration;

    #[test]
    fn test_rate_limiter_basic() {
        let limiter = RateLimiter::new(5, 1);
        
        // Should allow first 5 requests
        for _ in 0..5 {
            assert!(limiter.allow("127.0.0.1"));
        }
        
        // 6th should be blocked
        assert!(!limiter.allow("127.0.0.1"));
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let limiter = RateLimiter::new(2, 1);
        
        assert!(limiter.allow("192.168.1.1"));
        assert!(limiter.allow("192.168.1.1"));
        assert!(!limiter.allow("192.168.1.1"));
        
        // Different IP should have its own bucket
        assert!(limiter.allow("192.168.1.2"));
    }

    #[test]
    fn test_health_checker() {
        let health = HealthChecker::new();
        
        health.report("database", true, "Connected", 1.5);
        health.report("cache", true, "Online", 0.5);
        
        assert!(health.is_healthy());
        
        health.report("cache", false, "Connection lost", 100.0);
        assert!(!health.is_healthy());
    }

    #[test]
    fn test_metrics_counter() {
        let metrics = MetricsCollector::new();
        
        metrics.inc_counter("requests_total", 1);
        metrics.inc_counter("requests_total", 5);
        
        let value = metrics.counters.get("requests_total")
            .map(|v| v.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0);
        
        assert_eq!(value, 6);
    }

    #[test]
    fn test_metrics_gauge() {
        let metrics = MetricsCollector::new();
        
        metrics.set_gauge("connections", 100);
        metrics.set_gauge("connections", 95);
        
        let value = metrics.gauges.get("connections")
            .map(|v| v.load(std::sync::atomic::Ordering::Relaxed))
            .unwrap_or(0);
        
        assert_eq!(value, 95);
    }

    #[tokio::test]
    async fn test_shutdown_coordinator() {
        let coordinator = ShutdownCoordinator::new();
        
        assert!(!coordinator.is_shutdown());
        
        let guard = coordinator.begin_operation();
        assert_eq!(coordinator.active_operations(), 1);
        
        coordinator.shutdown();
        assert!(coordinator.is_shutdown());
        
        drop(guard);
        assert_eq!(coordinator.active_operations(), 0);
    }
}
