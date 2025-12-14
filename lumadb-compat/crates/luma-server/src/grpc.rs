
use tonic::{Request, Response, Status};
use luma_protocol_core::luma::v3::query_service_server::QueryService;
use luma_protocol_core::luma::v3::{QueryRequest, QueryResponse};
use std::sync::Arc;
use luma_protocol_core::query::executor::QueryExecutor;
use luma_protocol_core::ir::{Operation, QueryPlan};

pub struct GrpcQueryService {
    executor: Arc<QueryExecutor>,
}

impl GrpcQueryService {
    pub fn new(executor: Arc<QueryExecutor>) -> Self {
        Self { executor }
    }
}

#[tonic::async_trait]
impl QueryService for GrpcQueryService {
    type StreamExecuteStream = tokio_stream::wrappers::ReceiverStream<Result<luma_protocol_core::luma::v3::QueryResponse, tonic::Status>>;

    async fn execute(
        &self,
        request: tonic::Request<luma_protocol_core::luma::v3::QueryRequest>,
    ) -> Result<tonic::Response<luma_protocol_core::luma::v3::QueryResponse>, tonic::Status> {
        let req = request.into_inner();
        println!("Received gRPC Query: {}", req.query);
        
        // Parse req.query -> Operation (Mock for now)
        let op = Operation::Scan { 
            table: "mock".into(), 
            filter: None, 
            alias: None,
            columns: vec![] 
        };
        let plan = QueryPlan { steps: vec![op] };
        
        match self.executor.execute(plan).await {
            Ok(_results) => {
                // Map results to QueryResponse
                Ok(tonic::Response::new(luma_protocol_core::luma::v3::QueryResponse {
                    result: None, 
                }))
            },
            Err(e) => Err(tonic::Status::internal(e)),
        }
    }

    async fn stream_execute(
        &self,
        _request: tonic::Request<luma_protocol_core::luma::v3::QueryRequest>,
    ) -> Result<tonic::Response<Self::StreamExecuteStream>, tonic::Status> {
        Err(tonic::Status::unimplemented("Stream not implemented"))
    }
}
