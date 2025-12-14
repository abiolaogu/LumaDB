
use tonic::{Request, Response, Status};
use opentelemetry_proto::tonic::{
    collector::{
        metrics::v1::{
            metrics_service_server::MetricsService,
            ExportMetricsServiceRequest,
            ExportMetricsServiceResponse,
        },
        logs::v1::{
            logs_service_server::LogsService,
            ExportLogsServiceRequest,
            ExportLogsServiceResponse,
        },
        trace::v1::{
            trace_service_server::TraceService,
            ExportTraceServiceRequest,
            ExportTraceServiceResponse,
        },
    },
};
use std::sync::Arc;
use luma_protocol_core::QueryProcessor;

// Metrics Service
pub struct OtlpMetrics {
    processor: Arc<dyn QueryProcessor + Send + Sync>,
}

impl OtlpMetrics {
    pub fn new(processor: Arc<dyn QueryProcessor + Send + Sync>) -> Self {
        Self { processor }
    }
}

#[tonic::async_trait]
impl MetricsService for OtlpMetrics {
    async fn export(
        &self,
        request: Request<ExportMetricsServiceRequest>,
    ) -> Result<Response<ExportMetricsServiceResponse>, Status> {
        let req = request.into_inner();
        println!("Received OTLP Metrics: {} ResourceMetrics", req.resource_metrics.len());
        // TODO: Processing logic
        Ok(Response::new(ExportMetricsServiceResponse { partial_success: None }))
    }
}

// Logs Service
pub struct OtlpLogs {
    processor: Arc<dyn QueryProcessor + Send + Sync>,
}

impl OtlpLogs {
    pub fn new(processor: Arc<dyn QueryProcessor + Send + Sync>) -> Self {
        Self { processor }
    }
}

#[tonic::async_trait]
impl LogsService for OtlpLogs {
    async fn export(
        &self,
        request: Request<ExportLogsServiceRequest>,
    ) -> Result<Response<ExportLogsServiceResponse>, Status> {
        let req = request.into_inner();
        println!("Received OTLP Logs: {} ResourceLogs", req.resource_logs.len());
        Ok(Response::new(ExportLogsServiceResponse { partial_success: None }))
    }
}

// Trace Service (Simple Stub)
pub struct OtlpTraces {
    processor: Arc<dyn QueryProcessor + Send + Sync>,
}

impl OtlpTraces {
    pub fn new(processor: Arc<dyn QueryProcessor + Send + Sync>) -> Self {
        Self { processor }
    }
}

#[tonic::async_trait]
impl TraceService for OtlpTraces {
    async fn export(
        &self,
        request: Request<ExportTraceServiceRequest>,
    ) -> Result<Response<ExportTraceServiceResponse>, Status> {
         println!("Received OTLP Traces");
         Ok(Response::new(ExportTraceServiceResponse { partial_success: None }))
    }
}

pub async fn run(port: u16, processor: Arc<dyn QueryProcessor + Send + Sync>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = format!("0.0.0.0:{}", port).parse()?;
    println!("OTLP gRPC Server listening on {}", addr);
    
    use opentelemetry_proto::tonic::collector::{
        metrics::v1::metrics_service_server::MetricsServiceServer,
        logs::v1::logs_service_server::LogsServiceServer,
        trace::v1::trace_service_server::TraceServiceServer,
    };

    tonic::transport::Server::builder()
        .add_service(MetricsServiceServer::new(OtlpMetrics::new(processor.clone())))
        .add_service(LogsServiceServer::new(OtlpLogs::new(processor.clone())))
        .add_service(TraceServiceServer::new(OtlpTraces::new(processor)))
        .serve(addr)
        .await?;
        
    Ok(())
}
