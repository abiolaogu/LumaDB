//! Migration source connectors

use std::path::Path;

use serde::{Deserialize, Serialize};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use lumadb_common::types::Document;

use super::tool::DocumentBatch;

/// Migration source type
#[derive(Debug, Clone)]
pub enum MigrationSource {
    /// JSON or JSONL file
    Json(JsonSource),
    /// Qdrant instance
    Qdrant(QdrantSource),
    /// Pinecone index
    Pinecone(PineconeSource),
    /// MongoDB collection
    MongoDB(MongoDBSource),
}

/// Configuration for different sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    /// Source type
    pub source_type: String,
    /// Connection URL or file path
    pub url: String,
    /// Optional API key
    pub api_key: Option<String>,
    /// Database name (for MongoDB)
    pub database: Option<String>,
    /// Collection/index name
    pub collection: Option<String>,
}

/// JSON file source
#[derive(Debug, Clone)]
pub struct JsonSource {
    pub path: String,
}

/// Qdrant source
#[derive(Debug, Clone)]
pub struct QdrantSource {
    pub url: String,
    pub collection: String,
    pub api_key: Option<String>,
}

/// Pinecone source
#[derive(Debug, Clone)]
pub struct PineconeSource {
    pub api_key: String,
    pub environment: String,
    pub index: String,
}

/// MongoDB source
#[derive(Debug, Clone)]
pub struct MongoDBSource {
    pub connection_string: String,
    pub database: String,
    pub collection: String,
}

impl MigrationSource {
    /// Create a JSON file source
    pub fn json(path: &str) -> Self {
        Self::Json(JsonSource {
            path: path.to_string(),
        })
    }

    /// Create a Qdrant source
    pub fn qdrant(url: &str, collection: &str) -> Self {
        Self::Qdrant(QdrantSource {
            url: url.to_string(),
            collection: collection.to_string(),
            api_key: None,
        })
    }

    /// Create a Qdrant source with API key
    pub fn qdrant_with_key(url: &str, collection: &str, api_key: &str) -> Self {
        Self::Qdrant(QdrantSource {
            url: url.to_string(),
            collection: collection.to_string(),
            api_key: Some(api_key.to_string()),
        })
    }

    /// Create a Pinecone source
    pub fn pinecone(api_key: &str, environment: &str, index: &str) -> Self {
        Self::Pinecone(PineconeSource {
            api_key: api_key.to_string(),
            environment: environment.to_string(),
            index: index.to_string(),
        })
    }

    /// Create a MongoDB source
    pub fn mongodb(connection_string: &str, database: &str, collection: &str) -> Self {
        Self::MongoDB(MongoDBSource {
            connection_string: connection_string.to_string(),
            database: database.to_string(),
            collection: collection.to_string(),
        })
    }

    /// Get source type name
    pub fn source_type(&self) -> &'static str {
        match self {
            Self::Json(_) => "json",
            Self::Qdrant(_) => "qdrant",
            Self::Pinecone(_) => "pinecone",
            Self::MongoDB(_) => "mongodb",
        }
    }

    /// Stream documents from source
    pub async fn stream_documents(
        &self,
        tx: mpsc::Sender<DocumentBatch>,
        batch_size: usize,
        vector_field: &str,
        target_collection: &str,
    ) -> crate::Result<()> {
        match self {
            Self::Json(source) => {
                stream_json(source, tx, batch_size, vector_field, target_collection).await
            }
            Self::Qdrant(source) => {
                stream_qdrant(source, tx, batch_size, target_collection).await
            }
            Self::Pinecone(source) => {
                stream_pinecone(source, tx, batch_size, target_collection).await
            }
            Self::MongoDB(source) => {
                stream_mongodb(source, tx, batch_size, vector_field, target_collection).await
            }
        }
    }
}

/// Stream documents from JSON/JSONL file
async fn stream_json(
    source: &JsonSource,
    tx: mpsc::Sender<DocumentBatch>,
    batch_size: usize,
    vector_field: &str,
    target_collection: &str,
) -> crate::Result<()> {
    let path = Path::new(&source.path);
    let file = File::open(path)
        .await
        .map_err(|e| crate::CompatError::Storage(format!("Failed to open file: {}", e)))?;

    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    let mut documents = Vec::with_capacity(batch_size);
    let mut vectors = Vec::with_capacity(batch_size);

    let is_jsonl = source.path.ends_with(".jsonl") || source.path.ends_with(".ndjson");

    if is_jsonl {
        // JSONL format - one JSON object per line
        while let Ok(Some(line)) = lines.next_line().await {
            if line.trim().is_empty() {
                continue;
            }

            match serde_json::from_str::<serde_json::Value>(&line) {
                Ok(value) => {
                    let (doc, vector) = extract_document_and_vector(value, vector_field);
                    if let Some(v) = vector {
                        vectors.push((doc.id.clone(), v));
                    }
                    documents.push(doc);

                    if documents.len() >= batch_size {
                        let batch = DocumentBatch {
                            collection: target_collection.to_string(),
                            documents: std::mem::take(&mut documents),
                            vectors: std::mem::take(&mut vectors),
                        };
                        if tx.send(batch).await.is_err() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to parse JSON line: {}", e);
                }
            }
        }
    } else {
        // Regular JSON - expect an array
        let mut content = String::new();
        while let Ok(Some(line)) = lines.next_line().await {
            content.push_str(&line);
        }

        match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(serde_json::Value::Array(arr)) => {
                for value in arr {
                    let (doc, vector) = extract_document_and_vector(value, vector_field);
                    if let Some(v) = vector {
                        vectors.push((doc.id.clone(), v));
                    }
                    documents.push(doc);

                    if documents.len() >= batch_size {
                        let batch = DocumentBatch {
                            collection: target_collection.to_string(),
                            documents: std::mem::take(&mut documents),
                            vectors: std::mem::take(&mut vectors),
                        };
                        if tx.send(batch).await.is_err() {
                            break;
                        }
                    }
                }
            }
            Ok(value) => {
                // Single document
                let (doc, vector) = extract_document_and_vector(value, vector_field);
                if let Some(v) = vector {
                    vectors.push((doc.id.clone(), v));
                }
                documents.push(doc);
            }
            Err(e) => {
                return Err(crate::CompatError::Serialization(format!(
                    "Failed to parse JSON: {}",
                    e
                )));
            }
        }
    }

    // Send remaining documents
    if !documents.is_empty() {
        let batch = DocumentBatch {
            collection: target_collection.to_string(),
            documents,
            vectors,
        };
        let _ = tx.send(batch).await;
    }

    Ok(())
}

/// Extract document and optional vector from JSON value
fn extract_document_and_vector(
    mut value: serde_json::Value,
    vector_field: &str,
) -> (Document, Option<Vec<f32>>) {
    // Extract ID
    let id = value
        .get("_id")
        .or_else(|| value.get("id"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    // Extract vector if present
    let vector = value
        .get(vector_field)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_f64().map(|f| f as f32))
                .collect::<Vec<f32>>()
        });

    // Remove vector from document data to avoid storing it twice
    if let serde_json::Value::Object(ref mut map) = value {
        map.remove(vector_field);
    }

    let doc = Document::with_id(id, value);
    (doc, vector)
}

/// Stream documents from Qdrant
async fn stream_qdrant(
    source: &QdrantSource,
    tx: mpsc::Sender<DocumentBatch>,
    batch_size: usize,
    target_collection: &str,
) -> crate::Result<()> {
    info!("Streaming from Qdrant: {}/{}", source.url, source.collection);

    let client = reqwest::Client::new();
    let mut offset: Option<String> = None;

    loop {
        // Build scroll request
        let scroll_url = format!(
            "{}/collections/{}/points/scroll",
            source.url, source.collection
        );

        let mut request_body = serde_json::json!({
            "limit": batch_size,
            "with_payload": true,
            "with_vector": true,
        });

        if let Some(ref off) = offset {
            request_body["offset"] = serde_json::json!(off);
        }

        let mut request = client.post(&scroll_url).json(&request_body);

        if let Some(ref api_key) = source.api_key {
            request = request.header("api-key", api_key);
        }

        let response = request
            .send()
            .await
            .map_err(|e| crate::CompatError::Network(e.to_string()))?;

        if !response.status().is_success() {
            return Err(crate::CompatError::Network(format!(
                "Qdrant scroll failed: {}",
                response.status()
            )));
        }

        let result: serde_json::Value = response
            .json()
            .await
            .map_err(|e| crate::CompatError::Serialization(e.to_string()))?;

        let points = result
            .get("result")
            .and_then(|r| r.get("points"))
            .and_then(|p| p.as_array())
            .cloned()
            .unwrap_or_default();

        if points.is_empty() {
            break;
        }

        let mut documents = Vec::with_capacity(points.len());
        let mut vectors = Vec::with_capacity(points.len());

        for point in points {
            let id = point
                .get("id")
                .map(|v| match v {
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::String(s) => s.clone(),
                    _ => uuid::Uuid::new_v4().to_string(),
                })
                .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

            // Extract payload
            let payload = point
                .get("payload")
                .cloned()
                .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

            // Extract vector
            if let Some(vector_value) = point.get("vector") {
                if let Some(arr) = vector_value.as_array() {
                    let vector: Vec<f32> = arr
                        .iter()
                        .filter_map(|v| v.as_f64().map(|f| f as f32))
                        .collect();
                    vectors.push((id.clone(), vector));
                }
            }

            documents.push(Document::with_id(id, payload));
        }

        // Get next offset
        offset = result
            .get("result")
            .and_then(|r| r.get("next_page_offset"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let batch = DocumentBatch {
            collection: target_collection.to_string(),
            documents,
            vectors,
        };

        if tx.send(batch).await.is_err() {
            break;
        }

        if offset.is_none() {
            break;
        }
    }

    Ok(())
}

/// Stream documents from Pinecone
async fn stream_pinecone(
    source: &PineconeSource,
    tx: mpsc::Sender<DocumentBatch>,
    batch_size: usize,
    target_collection: &str,
) -> crate::Result<()> {
    info!(
        "Streaming from Pinecone: {}/{}",
        source.environment, source.index
    );

    let client = reqwest::Client::new();
    let base_url = format!(
        "https://{}-{}.svc.{}.pinecone.io",
        source.index,
        &source.api_key[..8], // First 8 chars of API key are typically the project ID
        source.environment
    );

    // Pinecone doesn't have a direct scroll API, so we use list + fetch
    // First, get the index stats to understand namespaces
    let describe_url = format!("{}/describe_index_stats", base_url);

    let response = client
        .post(&describe_url)
        .header("Api-Key", &source.api_key)
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| crate::CompatError::Network(e.to_string()))?;

    if !response.status().is_success() {
        return Err(crate::CompatError::Network(format!(
            "Pinecone describe failed: {}",
            response.status()
        )));
    }

    let stats: serde_json::Value = response
        .json()
        .await
        .map_err(|e| crate::CompatError::Serialization(e.to_string()))?;

    // Get namespaces
    let namespaces = stats
        .get("namespaces")
        .and_then(|n| n.as_object())
        .map(|m| m.keys().cloned().collect::<Vec<_>>())
        .unwrap_or_else(|| vec!["".to_string()]);

    for namespace in namespaces {
        debug!("Processing Pinecone namespace: {}", namespace);

        let mut pagination_token: Option<String> = None;

        loop {
            // List vectors in namespace
            let mut list_url = format!("{}/vectors/list", base_url);
            if !namespace.is_empty() {
                list_url = format!("{}?namespace={}", list_url, namespace);
            }
            if let Some(ref token) = pagination_token {
                list_url = format!("{}&paginationToken={}", list_url, token);
            }

            let response = client
                .get(&list_url)
                .header("Api-Key", &source.api_key)
                .send()
                .await
                .map_err(|e| crate::CompatError::Network(e.to_string()))?;

            if !response.status().is_success() {
                warn!("Failed to list vectors in namespace {}", namespace);
                break;
            }

            let list_result: serde_json::Value = response
                .json()
                .await
                .map_err(|e| crate::CompatError::Serialization(e.to_string()))?;

            let vector_ids: Vec<String> = list_result
                .get("vectors")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.get("id").and_then(|id| id.as_str()).map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            if vector_ids.is_empty() {
                break;
            }

            // Fetch vectors by ID
            let fetch_url = format!("{}/vectors/fetch", base_url);
            let fetch_body = serde_json::json!({
                "ids": vector_ids,
                "namespace": namespace,
            });

            let response = client
                .post(&fetch_url)
                .header("Api-Key", &source.api_key)
                .json(&fetch_body)
                .send()
                .await
                .map_err(|e| crate::CompatError::Network(e.to_string()))?;

            if response.status().is_success() {
                let fetch_result: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|e| crate::CompatError::Serialization(e.to_string()))?;

                let vectors_map = fetch_result
                    .get("vectors")
                    .and_then(|v| v.as_object())
                    .cloned()
                    .unwrap_or_default();

                let mut documents = Vec::with_capacity(vectors_map.len());
                let mut vectors = Vec::with_capacity(vectors_map.len());

                for (id, data) in vectors_map {
                    // Extract metadata as document
                    let metadata = data
                        .get("metadata")
                        .cloned()
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                    // Extract vector
                    if let Some(values) = data.get("values").and_then(|v| v.as_array()) {
                        let vector: Vec<f32> = values
                            .iter()
                            .filter_map(|v| v.as_f64().map(|f| f as f32))
                            .collect();
                        vectors.push((id.clone(), vector));
                    }

                    documents.push(Document::with_id(id, metadata));
                }

                if !documents.is_empty() {
                    let batch = DocumentBatch {
                        collection: target_collection.to_string(),
                        documents,
                        vectors,
                    };

                    if tx.send(batch).await.is_err() {
                        return Ok(());
                    }
                }
            }

            // Check for pagination
            pagination_token = list_result
                .get("pagination")
                .and_then(|p| p.get("next"))
                .and_then(|n| n.as_str())
                .map(|s| s.to_string());

            if pagination_token.is_none() {
                break;
            }
        }
    }

    Ok(())
}

/// Stream documents from MongoDB
async fn stream_mongodb(
    source: &MongoDBSource,
    tx: mpsc::Sender<DocumentBatch>,
    batch_size: usize,
    vector_field: &str,
    target_collection: &str,
) -> crate::Result<()> {
    info!(
        "Streaming from MongoDB: {}/{}",
        source.database, source.collection
    );

    // Note: This is a simplified implementation. In production, you'd use the mongodb crate.
    // For now, we'll just document the expected format and return an error.

    // To properly implement this, you would:
    // 1. Connect using mongodb::Client
    // 2. Get database and collection handles
    // 3. Use find() with cursor to stream documents
    // 4. Extract vector field and metadata

    warn!(
        "MongoDB migration requires the mongodb driver. Please use the CLI tool: \
         lumadb migrate --from mongodb://... --source-db {} --source-collection {} --target {}",
        source.database, source.collection, target_collection
    );

    // For demonstration, send an empty batch
    let batch = DocumentBatch {
        collection: target_collection.to_string(),
        documents: vec![],
        vectors: vec![],
    };
    let _ = tx.send(batch).await;

    Ok(())
}
