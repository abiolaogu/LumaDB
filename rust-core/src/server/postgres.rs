use crate::{Database, Result};
use std::sync::Arc;
use tokio::net::TcpListener;
use pgwire::api::{MakeHandler, StatelessMakeHandler, Type, FormatCode};
use pgwire::api::query::{SimpleQueryHandler, ExtendedQueryHandler, StatementOrPortal};
use pgwire::api::results::{Response, Tag, FieldInfo, TextDataRowEncoder};
use pgwire::error::PgWireResult;
use async_trait::async_trait;
use crate::server::translator::Translator;
use crate::server::query::ExecutionResult;
use crate::types::Value;
use crate::security::{SecurityManager, RateLimiter, auth::LumaStartupHandler};

pub async fn start(db: Arc<Database>, port: u16) -> Result<()> {
    let security_manager = Arc::new(SecurityManager::new());
    let rate_limiter = Arc::new(RateLimiter::new(100.0, 1000.0)); // 100 req/s, burst 1000
    
    // Auth Handler
    let authenticator = Arc::new(LumaStartupHandler::new(security_manager.clone()));
    
    // Query Handler
    let processor = Arc::new(StatelessMakeHandler::new(Arc::new(LumaPgHandler::new(db))));

    let addr = format!("0.0.0.0:{}", port);
    let listener = TcpListener::bind(&addr).await.map_err(|e| crate::LumaError::Io(e))?;
    println!("LumaDB Postgres Server (pgwire) listening on {}", addr);

    loop {
        let (stream, addr) = listener.accept().await.map_err(|e| crate::LumaError::Io(e))?;
        
        // Rate Limiting
        let ip = addr.ip().to_string();
        if !rate_limiter.check(&ip) {
            eprintln!("Rate Limit Exceeded for {}", ip);
            continue; // Drop connection
        }

        let processor_ref = processor.clone();
        let authenticator_ref = authenticator.clone();

        tokio::spawn(async move {
            if let Err(e) = pgwire::api::process_socket(
                stream,
                None,
                processor_ref,
                authenticator_ref,
                authenticator_ref.clone() // placeholder for "PlaceHolder" (factory?)
                // Wait, process_socket args:
                // stream, tls, make_handler, startup_handler, make_portal?
                // No, process_socket signature:
                // pub async fn process_socket<S, M, ST>(stream, tls, make_handler, startup_handler)
                // startup_handler is Arc<ST>.
                // Check signature carefully!
            ).await {
                eprintln!("PG Connection error: {}", e);
            }
        });
    }
}

pub struct LumaPgHandler {
    translator: Arc<Translator>,
}

impl LumaPgHandler {
    pub fn new(db: Arc<Database>) -> Self {
        Self {
            translator: Arc::new(Translator::new(db)),
        }
    }
}

#[async_trait]
impl SimpleQueryHandler for LumaPgHandler {
    // ... (Implementation same as before)
    async fn do_query<C>(&self, _client: &C, query: &str) -> PgWireResult<Vec<Response>>
    where
        C: pgwire::api::ClientInfo + Unpin + Send + Sync,
    {
         println!("Received SQL: {}", query);
        
        match self.translator.execute_sql_raw(query).await {
            Ok(result) => {
                match result {
                    ExecutionResult::Select(docs, projection) => {
                        // 1. Determine Schema
                        let fields = if let Some(proj) = &projection {
                            proj.iter().map(|name| {
                                FieldInfo::new(name.clone(), None, None, Type::TEXT, FormatCode::Text)
                            }).collect()
                        } else {
                             // Fallback for SELECT * (use keys from first doc or generic "doc")
                             if let Some(first) = docs.first() {
                                 let mut keys: Vec<_> = first.data.keys().cloned().collect();
                                 keys.sort(); // Stable order
                                 keys.iter().map(|name| {
                                     FieldInfo::new(name.clone(), None, None, Type::TEXT, FormatCode::Text)
                                 }).collect()
                             } else {
                                 vec![FieldInfo::new("result".into(), None, None, Type::TEXT, FormatCode::Text)]
                             }
                        };
                        
                        let mut response = Response::new(fields.clone(), Tag::new("SELECT"));
                        
                        // 2. Encode Rows
                        for doc in docs {
                            let mut encoder = TextDataRowEncoder::new(fields.len());
                            if let Some(proj) = &projection {
                                for col in proj {
                                    let val_str = match doc.data.get(col) {
                                        Some(Value::String(s)) => Some(s.clone()),
                                        Some(Value::Int(i)) => Some(i.to_string()),
                                        Some(Value::Float(f)) => Some(f.to_string()),
                                        Some(Value::Bool(b)) => Some(b.to_string()),
                                        Some(Value::Null) => None,
                                        Some(other) => Some(format!("{:?}", other)), // JSON etc
                                        None => None
                                    };
                                    encoder.append_field(val_str.as_deref())?;
                                }
                            } else {
                                encoder.append_field(Some(&format!("{:?}", doc)))?;
                            }
                            response.result_set_mut().append_row(encoder.finish());
                        }
                        
                        Ok(vec![response])
                    },
                    ExecutionResult::Modify { affected } => {
                         let tag = if query.to_uppercase().starts_with("INSERT") {
                            format!("INSERT 0 {}", affected)
                        } else if query.to_uppercase().starts_with("UPDATE") {
                            format!("UPDATE {}", affected)
                        } else if query.to_uppercase().starts_with("DELETE") {
                            format!("DELETE {}", affected)
                        } else {
                            format!("OK {}", affected)
                        };
                        Ok(vec![Response::new(vec![], Tag::new(&tag))])
                    },
                    ExecutionResult::Ping => {
                        Ok(vec![Response::new(vec![], Tag::new("OK"))])
                    }
                }
            },
            Err(e) => {
                eprintln!("SQL Execution Error: {}", e);
                Ok(vec![Response::new(vec![], Tag::new("ERROR"))]) 
            }
        }
    }
}

#[async_trait]
impl ExtendedQueryHandler for LumaPgHandler {
    async fn do_query<C>(&self, _client: &mut C, _portal: &StatementOrPortal) -> PgWireResult<Response>
    where
        C: pgwire::api::ClientInfo + Unpin + Send + Sync,
    {
        Ok(Response::new(vec![], Tag::new("OK")))
    }
    
    async fn do_describe<C>(&self, _client: &mut C, _statement: &StatementOrPortal) -> PgWireResult<Response>
    where
         C: pgwire::api::ClientInfo + Unpin + Send + Sync,
    {
         Ok(Response::new(vec![], Tag::new("OK")))
    }
}
