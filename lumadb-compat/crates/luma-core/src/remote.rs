use crate::{QueryProcessor, QueryRequest, QueryResult, Value, Result, ProtocolError};
use async_trait::async_trait;
use tonic::transport::Channel;

pub mod pb {
    tonic::include_proto!("luma.v1");
}

use pb::luma_service_client::LumaServiceClient;
use pb::{QueryRequest as PbQueryRequest};

pub struct RemoteQueryProcessor {
    client: LumaServiceClient<Channel>,
}

impl RemoteQueryProcessor {
    pub async fn connect(addr: String) -> Result<Self> {
        let client = LumaServiceClient::connect(addr)
            .await
            .map_err(|e| ProtocolError::Io(std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e)))?;
        
        Ok(Self { client })
    }
}

#[async_trait]
impl QueryProcessor for RemoteQueryProcessor {
    async fn process(&self, request: QueryRequest) -> Result<QueryResult> {
        let mut client = self.client.clone();

        // Convert request.params to JSON bytes for payload
        let payload = serde_json::to_vec(&request.params)
            .map_err(|e| ProtocolError::TypeConversion(e.to_string()))?;

        let req = tonic::Request::new(PbQueryRequest {
            collection: "".to_string(), // TODO: Extract from query or request?
            query: request.query,
            dialect: "luma-ir".to_string(), // Or "sql-pg" etc depending on source
            payload,
        });

        let response = client.execute(req).await
            .map_err(|e| ProtocolError::Internal(format!("gRPC Error: {}", e)))?
            .into_inner();

        if !response.success {
            return Err(ProtocolError::Internal(response.error));
        }

        // Parse result
        // Assuming result is JSON array of rows for now
        if response.content_type == "json" {
             let rows: Vec<Vec<Value>> = serde_json::from_slice(&response.result)
                .map_err(|e| ProtocolError::TypeConversion(format!("Failed to parse result: {}", e)))?;
             
             Ok(QueryResult {
                 rows,
                 row_count: response.rows_affected as usize,
             })
        } else {
             Ok(QueryResult {
                 rows: vec![],
                 row_count: 0,
             })
        }
    }
}
