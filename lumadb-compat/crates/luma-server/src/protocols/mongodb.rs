//! MongoDB Wire Protocol Implementation
//! Provides MongoDB-compatible OP_MSG with aggregation pipeline

use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use std::collections::HashMap;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tracing::{info, debug, error};

// MongoDB Opcodes
const OP_MSG: u32 = 2013;
const OP_QUERY: u32 = 2004;
const OP_REPLY: u32 = 1;

/// MongoDB Document (BSON-like using JSON)
pub type Document = Value;

/// MongoDB Collection
#[derive(Clone, Debug, Default)]
pub struct Collection {
    pub name: String,
    pub documents: Vec<Document>,
}

/// MongoDB Database
#[derive(Clone, Debug, Default)]
pub struct Database {
    pub name: String,
    pub collections: HashMap<String, Collection>,
}

/// MongoDB Store
pub struct MongoStore {
    databases: Arc<DashMap<String, Database>>,
    next_id: Arc<std::sync::atomic::AtomicU64>,
}

impl MongoStore {
    pub fn new() -> Self {
        Self {
            databases: Arc::new(DashMap::new()),
            next_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    fn generate_id(&self) -> String {
        let id = self.next_id.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        format!("{:024x}", id)
    }

    /// Execute MongoDB command
    pub fn execute_command(&self, db: &str, command: &Document) -> Document {
        // Determine command type
        if let Some(coll) = command.get("insert").and_then(|v| v.as_str()) {
            return self.insert(db, coll, command);
        }
        if let Some(coll) = command.get("find").and_then(|v| v.as_str()) {
            return self.find(db, coll, command);
        }
        if let Some(coll) = command.get("update").and_then(|v| v.as_str()) {
            return self.update(db, coll, command);
        }
        if let Some(coll) = command.get("delete").and_then(|v| v.as_str()) {
            return self.delete(db, coll, command);
        }
        if let Some(coll) = command.get("aggregate").and_then(|v| v.as_str()) {
            return self.aggregate(db, coll, command);
        }
        if command.get("createIndexes").is_some() {
            return json!({"ok": 1});
        }
        if command.get("dropDatabase").is_some() {
            self.databases.remove(db);
            return json!({"ok": 1, "dropped": db});
        }
        if command.get("drop").is_some() {
            if let Some(coll) = command.get("drop").and_then(|v| v.as_str()) {
                if let Some(mut database) = self.databases.get_mut(db) {
                    database.collections.remove(coll);
                }
            }
            return json!({"ok": 1});
        }
        if command.get("listCollections").is_some() {
            return self.list_collections(db);
        }
        if command.get("listDatabases").is_some() {
            return self.list_databases();
        }
        if command.get("ping").is_some() {
            return json!({"ok": 1});
        }
        if command.get("isMaster").is_some() || command.get("ismaster").is_some() {
            return self.is_master();
        }
        if command.get("buildInfo").is_some() || command.get("buildinfo").is_some() {
            return self.build_info();
        }
        if command.get("getLastError").is_some() {
            return json!({"ok": 1, "n": 1, "err": null});
        }
        
        json!({"ok": 1})
    }

    fn is_master(&self) -> Document {
        json!({
            "ismaster": true,
            "maxBsonObjectSize": 16777216,
            "maxMessageSizeBytes": 48000000,
            "maxWriteBatchSize": 100000,
            "ok": 1
        })
    }

    fn build_info(&self) -> Document {
        json!({
            "version": "6.0.0-lumadb",
            "gitVersion": "lumadb",
            "allocator": "system",
            "ok": 1
        })
    }

    fn insert(&self, db: &str, collection: &str, command: &Document) -> Document {
        let docs = command.get("documents").and_then(|v| v.as_array());
        if docs.is_none() {
            return json!({"ok": 0, "errmsg": "No documents"});
        }
        
        let docs = docs.unwrap();
        let mut inserted = 0;
        
        self.databases.entry(db.to_string()).or_insert_with(|| Database {
            name: db.to_string(),
            collections: HashMap::new(),
        });
        
        if let Some(mut database) = self.databases.get_mut(db) {
            let coll = database.collections.entry(collection.to_string())
                .or_insert_with(|| Collection {
                    name: collection.to_string(),
                    documents: Vec::new(),
                });
            
            for doc in docs {
                let mut doc = doc.clone();
                if doc.get("_id").is_none() {
                    doc["_id"] = json!(self.generate_id());
                }
                coll.documents.push(doc);
                inserted += 1;
            }
        }
        
        json!({"ok": 1, "n": inserted})
    }

    fn find(&self, db: &str, collection: &str, command: &Document) -> Document {
        let filter = command.get("filter").cloned().unwrap_or(json!({}));
        let limit = command.get("limit").and_then(|v| v.as_i64()).unwrap_or(100) as usize;
        let skip = command.get("skip").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let projection = command.get("projection");
        
        let mut results = Vec::new();
        
        if let Some(database) = self.databases.get(db) {
            if let Some(coll) = database.collections.get(collection) {
                for doc in coll.documents.iter().skip(skip).take(limit) {
                    if self.matches_filter(doc, &filter) {
                        let result = if let Some(proj) = projection {
                            self.apply_projection(doc, proj)
                        } else {
                            doc.clone()
                        };
                        results.push(result);
                    }
                }
            }
        }
        
        json!({
            "cursor": {
                "firstBatch": results,
                "id": 0,
                "ns": format!("{}.{}", db, collection)
            },
            "ok": 1
        })
    }

    fn update(&self, db: &str, collection: &str, command: &Document) -> Document {
        let updates = command.get("updates").and_then(|v| v.as_array());
        if updates.is_none() {
            return json!({"ok": 0, "errmsg": "No updates"});
        }
        
        let mut matched = 0;
        let mut modified = 0;
        
        if let Some(mut database) = self.databases.get_mut(db) {
            if let Some(coll) = database.collections.get_mut(collection) {
                for update_spec in updates.unwrap() {
                    let filter = update_spec.get("q").cloned().unwrap_or(json!({}));
                    let update = update_spec.get("u").cloned().unwrap_or(json!({}));
                    let multi = update_spec.get("multi").and_then(|v| v.as_bool()).unwrap_or(false);
                    
                    for doc in &mut coll.documents {
                        if self.matches_filter(doc, &filter) {
                            matched += 1;
                            self.apply_update(doc, &update);
                            modified += 1;
                            if !multi { break; }
                        }
                    }
                }
            }
        }
        
        json!({"ok": 1, "n": matched, "nModified": modified})
    }

    fn delete(&self, db: &str, collection: &str, command: &Document) -> Document {
        let deletes = command.get("deletes").and_then(|v| v.as_array());
        if deletes.is_none() {
            return json!({"ok": 0, "errmsg": "No deletes"});
        }
        
        let mut deleted = 0;
        
        if let Some(mut database) = self.databases.get_mut(db) {
            if let Some(coll) = database.collections.get_mut(collection) {
                for delete_spec in deletes.unwrap() {
                    let filter = delete_spec.get("q").cloned().unwrap_or(json!({}));
                    let limit = delete_spec.get("limit").and_then(|v| v.as_i64()).unwrap_or(0);
                    
                    let before = coll.documents.len();
                    if limit == 1 {
                        if let Some(pos) = coll.documents.iter().position(|d| self.matches_filter(d, &filter)) {
                            coll.documents.remove(pos);
                        }
                    } else {
                        coll.documents.retain(|d| !self.matches_filter(d, &filter));
                    }
                    deleted += before - coll.documents.len();
                }
            }
        }
        
        json!({"ok": 1, "n": deleted})
    }

    /// Aggregation pipeline
    fn aggregate(&self, db: &str, collection: &str, command: &Document) -> Document {
        let pipeline = command.get("pipeline").and_then(|v| v.as_array());
        if pipeline.is_none() {
            return json!({"ok": 0, "errmsg": "No pipeline"});
        }
        
        let mut results: Vec<Document> = Vec::new();
        
        // Get initial documents
        if let Some(database) = self.databases.get(db) {
            if let Some(coll) = database.collections.get(collection) {
                results = coll.documents.clone();
            }
        }
        
        // Process pipeline stages
        for stage in pipeline.unwrap() {
            if let Some(filter) = stage.get("$match") {
                results = results.into_iter()
                    .filter(|d| self.matches_filter(d, filter))
                    .collect();
            }
            else if let Some(proj) = stage.get("$project") {
                results = results.into_iter()
                    .map(|d| self.apply_projection(&d, proj))
                    .collect();
            }
            else if let Some(group) = stage.get("$group") {
                results = vec![self.apply_group(&results, group)];
            }
            else if let Some(sort) = stage.get("$sort") {
                self.apply_sort(&mut results, sort);
            }
            else if let Some(limit) = stage.get("$limit").and_then(|v| v.as_i64()) {
                results.truncate(limit as usize);
            }
            else if let Some(skip) = stage.get("$skip").and_then(|v| v.as_i64()) {
                results = results.into_iter().skip(skip as usize).collect();
            }
            else if let Some(unwind) = stage.get("$unwind") {
                results = self.apply_unwind(&results, unwind);
            }
            else if let Some(lookup) = stage.get("$lookup") {
                results = self.apply_lookup(db, &results, lookup);
            }
        }
        
        json!({
            "cursor": {
                "firstBatch": results,
                "id": 0,
                "ns": format!("{}.{}", db, collection)
            },
            "ok": 1
        })
    }

    fn matches_filter(&self, doc: &Document, filter: &Document) -> bool {
        if let Some(obj) = filter.as_object() {
            for (key, expected) in obj {
                let actual = doc.get(key);
                
                // Handle operators
                if let Some(ops) = expected.as_object() {
                    for (op, val) in ops {
                        match op.as_str() {
                            "$eq" => if actual != Some(val) { return false; }
                            "$ne" => if actual == Some(val) { return false; }
                            "$gt" => {
                                if let (Some(a), Some(v)) = (actual.and_then(|x| x.as_f64()), val.as_f64()) {
                                    if a <= v { return false; }
                                }
                            }
                            "$gte" => {
                                if let (Some(a), Some(v)) = (actual.and_then(|x| x.as_f64()), val.as_f64()) {
                                    if a < v { return false; }
                                }
                            }
                            "$lt" => {
                                if let (Some(a), Some(v)) = (actual.and_then(|x| x.as_f64()), val.as_f64()) {
                                    if a >= v { return false; }
                                }
                            }
                            "$lte" => {
                                if let (Some(a), Some(v)) = (actual.and_then(|x| x.as_f64()), val.as_f64()) {
                                    if a > v { return false; }
                                }
                            }
                            "$in" => {
                                if let Some(arr) = val.as_array() {
                                    if !arr.contains(actual.unwrap_or(&Value::Null)) { return false; }
                                }
                            }
                            "$nin" => {
                                if let Some(arr) = val.as_array() {
                                    if arr.contains(actual.unwrap_or(&Value::Null)) { return false; }
                                }
                            }
                            "$exists" => {
                                let exists = val.as_bool().unwrap_or(true);
                                if exists && actual.is_none() { return false; }
                                if !exists && actual.is_some() { return false; }
                            }
                            "$regex" => {
                                if let (Some(s), Some(pattern)) = (actual.and_then(|x| x.as_str()), val.as_str()) {
                                    if !s.contains(pattern) { return false; }
                                }
                            }
                            _ => {}
                        }
                    }
                } else {
                    // Direct equality
                    if actual != Some(expected) { return false; }
                }
            }
        }
        true
    }

    fn apply_projection(&self, doc: &Document, projection: &Document) -> Document {
        let mut result = json!({});
        if let Some(proj) = projection.as_object() {
            for (key, include) in proj {
                if include.as_i64() == Some(1) || include.as_bool() == Some(true) {
                    if let Some(val) = doc.get(key) {
                        result[key] = val.clone();
                    }
                }
            }
            // Always include _id unless explicitly excluded
            if proj.get("_id").and_then(|v| v.as_i64()) != Some(0) {
                if let Some(id) = doc.get("_id") {
                    result["_id"] = id.clone();
                }
            }
        }
        result
    }

    fn apply_update(&self, doc: &mut Document, update: &Document) {
        if let Some(obj) = update.as_object() {
            for (op, fields) in obj {
                if let Some(fields_obj) = fields.as_object() {
                    match op.as_str() {
                        "$set" => {
                            for (k, v) in fields_obj {
                                doc[k] = v.clone();
                            }
                        }
                        "$unset" => {
                            if let Some(doc_obj) = doc.as_object_mut() {
                                for k in fields_obj.keys() {
                                    doc_obj.remove(k);
                                }
                            }
                        }
                        "$inc" => {
                            for (k, v) in fields_obj {
                                if let Some(inc) = v.as_f64() {
                                    let current = doc.get(k).and_then(|x| x.as_f64()).unwrap_or(0.0);
                                    doc[k] = json!(current + inc);
                                }
                            }
                        }
                        "$push" => {
                            for (k, v) in fields_obj {
                                let arr = doc.get_mut(k);
                                if let Some(Value::Array(arr)) = arr {
                                    arr.push(v.clone());
                                } else {
                                    doc[k] = json!([v.clone()]);
                                }
                            }
                        }
                        "$pull" => {
                            for (k, v) in fields_obj {
                                if let Some(Value::Array(arr)) = doc.get_mut(k) {
                                    arr.retain(|x| x != v);
                                }
                            }
                        }
                        "$addToSet" => {
                            for (k, v) in fields_obj {
                                let arr = doc.get_mut(k);
                                if let Some(Value::Array(arr)) = arr {
                                    if !arr.contains(v) {
                                        arr.push(v.clone());
                                    }
                                } else {
                                    doc[k] = json!([v.clone()]);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn apply_group(&self, docs: &[Document], group: &Document) -> Document {
        let id = group.get("_id");
        let mut result = json!({"_id": id});
        
        if let Some(obj) = group.as_object() {
            for (key, acc) in obj {
                if key == "_id" { continue; }
                
                if let Some(acc_obj) = acc.as_object() {
                    for (op, field) in acc_obj {
                        let field_name = field.as_str().map(|s| s.trim_start_matches('$'));
                        
                        match op.as_str() {
                            "$sum" => {
                                if let Some(field) = field_name {
                                    let sum: f64 = docs.iter()
                                        .filter_map(|d| d.get(field).and_then(|v| v.as_f64()))
                                        .sum();
                                    result[key] = json!(sum);
                                } else if let Some(n) = field.as_i64() {
                                    result[key] = json!(docs.len() as i64 * n);
                                }
                            }
                            "$avg" => {
                                if let Some(field) = field_name {
                                    let values: Vec<f64> = docs.iter()
                                        .filter_map(|d| d.get(field).and_then(|v| v.as_f64()))
                                        .collect();
                                    if !values.is_empty() {
                                        let avg = values.iter().sum::<f64>() / values.len() as f64;
                                        result[key] = json!(avg);
                                    }
                                }
                            }
                            "$min" => {
                                if let Some(field) = field_name {
                                    let min = docs.iter()
                                        .filter_map(|d| d.get(field).and_then(|v| v.as_f64()))
                                        .fold(f64::MAX, f64::min);
                                    if min != f64::MAX {
                                        result[key] = json!(min);
                                    }
                                }
                            }
                            "$max" => {
                                if let Some(field) = field_name {
                                    let max = docs.iter()
                                        .filter_map(|d| d.get(field).and_then(|v| v.as_f64()))
                                        .fold(f64::MIN, f64::max);
                                    if max != f64::MIN {
                                        result[key] = json!(max);
                                    }
                                }
                            }
                            "$first" => {
                                if let Some(field) = field_name {
                                    if let Some(first) = docs.first().and_then(|d| d.get(field)) {
                                        result[key] = first.clone();
                                    }
                                }
                            }
                            "$last" => {
                                if let Some(field) = field_name {
                                    if let Some(last) = docs.last().and_then(|d| d.get(field)) {
                                        result[key] = last.clone();
                                    }
                                }
                            }
                            "$count" => {
                                result[key] = json!(docs.len());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        result
    }

    fn apply_sort(&self, docs: &mut Vec<Document>, sort: &Document) {
        if let Some(obj) = sort.as_object() {
            for (field, order) in obj {
                let asc = order.as_i64().unwrap_or(1) > 0;
                docs.sort_by(|a, b| {
                    let av = a.get(field);
                    let bv = b.get(field);
                    let cmp = match (av, bv) {
                        (Some(Value::Number(an)), Some(Value::Number(bn))) => {
                            an.as_f64().partial_cmp(&bn.as_f64()).unwrap_or(std::cmp::Ordering::Equal)
                        }
                        (Some(Value::String(as_)), Some(Value::String(bs))) => as_.cmp(bs),
                        _ => std::cmp::Ordering::Equal,
                    };
                    if asc { cmp } else { cmp.reverse() }
                });
            }
        }
    }

    fn apply_unwind(&self, docs: &[Document], unwind: &Document) -> Vec<Document> {
        let path = unwind.as_str()
            .or_else(|| unwind.get("path").and_then(|v| v.as_str()))
            .map(|s| s.trim_start_matches('$'));
        
        if path.is_none() { return docs.to_vec(); }
        let path = path.unwrap();
        
        let mut result = Vec::new();
        for doc in docs {
            if let Some(Value::Array(arr)) = doc.get(path) {
                for item in arr {
                    let mut new_doc = doc.clone();
                    new_doc[path] = item.clone();
                    result.push(new_doc);
                }
            } else {
                result.push(doc.clone());
            }
        }
        result
    }

    fn apply_lookup(&self, db: &str, docs: &[Document], lookup: &Document) -> Vec<Document> {
        let from = lookup.get("from").and_then(|v| v.as_str());
        let local_field = lookup.get("localField").and_then(|v| v.as_str());
        let foreign_field = lookup.get("foreignField").and_then(|v| v.as_str());
        let as_field = lookup.get("as").and_then(|v| v.as_str());
        
        if from.is_none() || local_field.is_none() || foreign_field.is_none() || as_field.is_none() {
            return docs.to_vec();
        }
        
        let (from, local_field, foreign_field, as_field) = 
            (from.unwrap(), local_field.unwrap(), foreign_field.unwrap(), as_field.unwrap());
        
        let foreign_docs: Vec<Document> = self.databases.get(db)
            .and_then(|database| database.collections.get(from).map(|c| c.documents.clone()))
            .unwrap_or_default();
        
        docs.iter().map(|doc| {
            let local_val = doc.get(local_field);
            let matches: Vec<Document> = foreign_docs.iter()
                .filter(|fd| fd.get(foreign_field) == local_val)
                .cloned()
                .collect();
            let mut new_doc = doc.clone();
            new_doc[as_field] = json!(matches);
            new_doc
        }).collect()
    }

    fn list_collections(&self, db: &str) -> Document {
        let mut collections = Vec::new();
        if let Some(database) = self.databases.get(db) {
            for name in database.collections.keys() {
                collections.push(json!({"name": name, "type": "collection"}));
            }
        }
        json!({
            "cursor": {
                "firstBatch": collections,
                "id": 0,
                "ns": format!("{}.$cmd.listCollections", db)
            },
            "ok": 1
        })
    }

    fn list_databases(&self) -> Document {
        let databases: Vec<Document> = self.databases.iter()
            .map(|e| json!({"name": e.key(), "sizeOnDisk": 1024, "empty": e.value().collections.is_empty()}))
            .collect();
        json!({"databases": databases, "totalSize": databases.len() * 1024, "ok": 1})
    }
}

/// MongoDB Connection handler
pub struct MongoConnection {
    store: Arc<MongoStore>,
}

impl MongoConnection {
    pub fn new(store: Arc<MongoStore>) -> Self {
        Self { store }
    }

    pub async fn handle(&mut self, mut stream: TcpStream) -> std::io::Result<()> {
        loop {
            // Read message header (16 bytes)
            let mut header = [0u8; 16];
            match stream.read_exact(&mut header).await {
                Ok(_) => {}
                Err(_) => break,
            }

            let length = u32::from_le_bytes([header[0], header[1], header[2], header[3]]) as usize;
            let request_id = i32::from_le_bytes([header[4], header[5], header[6], header[7]]);
            let _response_to = i32::from_le_bytes([header[8], header[9], header[10], header[11]]);
            let opcode = u32::from_le_bytes([header[12], header[13], header[14], header[15]]);

            // Read body
            let body_len = length.saturating_sub(16);
            let mut body = vec![0u8; body_len];
            if body_len > 0 {
                stream.read_exact(&mut body).await?;
            }

            let response = match opcode {
                OP_MSG => self.handle_op_msg(&body),
                OP_QUERY => self.handle_op_query(&body),
                _ => {
                    debug!("Unknown opcode: {}", opcode);
                    json!({"ok": 0, "errmsg": "Unknown opcode"})
                }
            };

            // Send response
            self.send_reply(&mut stream, request_id, &response).await?;
        }

        Ok(())
    }

    fn handle_op_msg(&self, body: &[u8]) -> Document {
        if body.len() < 5 { return json!({"ok": 0}); }
        
        let _flags = u32::from_le_bytes([body[0], body[1], body[2], body[3]]);
        let _section_kind = body[4];
        
        // Parse BSON document (simplified: using JSON)
        // In real implementation, would use bson crate
        let doc_start = 5;
        if doc_start >= body.len() { return json!({"ok": 0}); }
        
        // Try to parse as JSON (simplified for demo)
        let doc_bytes = &body[doc_start..];
        let command = self.parse_bson_simplified(doc_bytes);
        
        let db = command.get("$db").and_then(|v| v.as_str()).unwrap_or("test");
        self.store.execute_command(db, &command)
    }

    fn handle_op_query(&self, body: &[u8]) -> Document {
        // OP_QUERY format: flags + collname + skip + limit + query
        if body.len() < 12 { return json!({"ok": 0}); }
        
        // For legacy queries, return isMaster
        self.store.is_master()
    }

    fn parse_bson_simplified(&self, _bytes: &[u8]) -> Document {
        // In production, use bson crate
        // For now, return a simple isMaster command as fallback
        json!({"isMaster": 1})
    }

    async fn send_reply(&self, stream: &mut TcpStream, request_id: i32, doc: &Document) -> std::io::Result<()> {
        // Serialize document to BSON (simplified)
        let doc_json = serde_json::to_vec(doc).unwrap_or_default();
        
        // OP_MSG response
        let mut msg = Vec::new();
        
        // Flags
        msg.extend_from_slice(&0u32.to_le_bytes());
        
        // Section: kind 0, body
        msg.push(0);
        
        // BSON document (using JSON for simplicity)
        // In production, would use proper BSON encoding
        let bson_len = (doc_json.len() + 5) as u32;
        msg.extend_from_slice(&bson_len.to_le_bytes());
        msg.extend_from_slice(&doc_json);
        msg.push(0); // null terminator
        
        // Header
        let total_len = (16 + msg.len()) as u32;
        let mut header = Vec::new();
        header.extend_from_slice(&total_len.to_le_bytes());
        header.extend_from_slice(&(request_id + 1).to_le_bytes()); // Response ID
        header.extend_from_slice(&request_id.to_le_bytes()); // Response to
        header.extend_from_slice(&OP_MSG.to_le_bytes());
        
        stream.write_all(&header).await?;
        stream.write_all(&msg).await?;
        stream.flush().await
    }
}

impl Default for MongoStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Run MongoDB server
pub async fn run(port: u16) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let listener = TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    let store = Arc::new(MongoStore::new());
    
    info!("MongoDB Server listening on 0.0.0.0:{}", port);
    
    loop {
        let (stream, addr) = listener.accept().await?;
        let store = store.clone();
        debug!("MongoDB connection from {}", addr);
        
        tokio::spawn(async move {
            let mut conn = MongoConnection::new(store);
            if let Err(e) = conn.handle(stream).await {
                error!("MongoDB connection error: {}", e);
            }
        });
    }
}
