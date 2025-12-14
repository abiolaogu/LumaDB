use crate::{QueryProcessor, QueryRequest, QueryResult, Value, Result, ProtocolError};
use async_trait::async_trait;
use tonic::transport::Channel;

pub mod pb {
    tonic::include_proto!("luma.v3");
}

use pb::query_service_client::QueryServiceClient;
use pb::{QueryRequest as PbQueryRequest};

pub struct RemoteQueryProcessor {
    client: QueryServiceClient<Channel>,
}

impl RemoteQueryProcessor {
    pub async fn connect(addr: String) -> Result<Self> {
        let client = QueryServiceClient::connect(addr)
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
        // Payload removed in v3

        let req = tonic::Request::new(PbQueryRequest {
            query: request.query,
            format: "luma-ir".to_string(),
        });

        let response = client.execute(req).await
            .map_err(|e| ProtocolError::Internal(format!("gRPC Error: {}", e)))?
            .into_inner();

        match response.result {
            Some(pb::query_response::Result::Vector(v)) => {
                let rows = v.matches.iter().map(|m| {
                     vec![Value::Int64(m.id as i64), Value::Float64(m.score as f64)]
                }).collect();
                Ok(QueryResult { rows, row_count: v.matches.len() })
            },
            Some(pb::query_response::Result::Scalar(s)) => {
                let rows = vec![vec![Value::Float64(s.value), Value::Text(s.label)]];
                Ok(QueryResult { rows, row_count: 1 })
            },
            Some(pb::query_response::Result::Error(e)) => {
                 Err(ProtocolError::Internal(e.message))
            },
            None => Ok(QueryResult { rows: vec![], row_count: 0 }),
        }
    }
}
