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

    // Helper for Superset/PostgreSQL system catalog queries
    fn handle_system_query(&self, query: &str) -> Option<Response> {
        let lower_query = query.to_lowercase();

        if lower_query.contains("pg_catalog.pg_database") {
            let fields = vec![
                FieldInfo::new("datname".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("oid".into(), None, None, Type::INT4, FormatCode::Text),
            ];
            let mut response = Response::new(fields, Tag::new("SELECT 1"));
            let mut encoder = TextDataRowEncoder::new(2);
            let _ = encoder.append_field(Some("lumadb"));
            let _ = encoder.append_field(Some("1"));
            response.result_set_mut().append_row(encoder.finish());
            Some(response)
        } else if lower_query.contains("pg_catalog.pg_tables") || lower_query.contains("pg_catalog.pg_class") {
            let fields = vec![
                FieldInfo::new("tablename".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("schemaname".into(), None, None, Type::TEXT, FormatCode::Text),
            ];
            let response = Response::new(fields, Tag::new("SELECT 0"));
            Some(response)
        } else if lower_query.contains("pg_catalog.pg_namespace") {
            let fields = vec![
                FieldInfo::new("nspname".into(), None, None, Type::TEXT, FormatCode::Text),
            ];
            let mut response = Response::new(fields, Tag::new("SELECT 1"));
            let mut encoder = TextDataRowEncoder::new(1);
            let _ = encoder.append_field(Some("public"));
            response.result_set_mut().append_row(encoder.finish());
            Some(response)
        } else if lower_query.contains("pg_catalog.pg_type") {
            let fields = vec![
                FieldInfo::new("typname".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("oid".into(), None, None, Type::INT4, FormatCode::Text),
                FieldInfo::new("typrelid".into(), None, None, Type::INT4, FormatCode::Text), // For composite types
                FieldInfo::new("typnamespace".into(), None, None, Type::INT4, FormatCode::Text), // For namespace
                FieldInfo::new("typtype".into(), None, None, Type::CHAR, FormatCode::Text), // 'b' for base, 'c' for composite
                FieldInfo::new("typcategory".into(), None, None, Type::CHAR, FormatCode::Text), // 'S' for string, 'N' for numeric
                FieldInfo::new("typispreferred".into(), None, None, Type::BOOL, FormatCode::Text),
                FieldInfo::new("typlen".into(), None, None, Type::INT2, FormatCode::Text),
                FieldInfo::new("typbyval".into(), None, None, Type::BOOL, FormatCode::Text),
                FieldInfo::new("typalign".into(), None, None, Type::CHAR, FormatCode::Text),
                FieldInfo::new("typdelim".into(), None, None, Type::CHAR, FormatCode::Text),
                FieldInfo::new("typcollation".into(), None, None, Type::INT4, FormatCode::Text),
                FieldInfo::new("typdefault".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("typndims".into(), None, None, Type::INT4, FormatCode::Text),
                FieldInfo::new("typmodin".into(), None, None, Type::INT4, FormatCode::Text),
                FieldInfo::new("typmodout".into(), None, None, Type::INT4, FormatCode::Text),
                FieldInfo::new("typarray".into(), None, None, Type::INT4, FormatCode::Text),
            ];
            let mut response = Response::new(fields, Tag::new("SELECT 6"));

            // Add common PostgreSQL types for Superset
            let types_to_add = vec![
                ("text", "25", "0", "2200", "b", "S", "t", "-1", "f", "c", ",", "0", "NULL", "0", "0", "0", "1009"),
                ("int4", "23", "0", "2200", "b", "N", "t", "4", "t", "i", ",", "0", "NULL", "0", "0", "0", "1007"),
                ("float8", "701", "0", "2200", "b", "N", "t", "8", "t", "d", ",", "0", "NULL", "0", "0", "0", "1022"),
                ("bool", "16", "0", "2200", "b", "B", "t", "1", "t", "c", ",", "0", "NULL", "0", "0", "0", "1000"),
                ("jsonb", "3802", "0", "2200", "b", "U", "f", "-1", "f", "i", ",", "0", "NULL", "0", "0", "0", "3807"),
                ("varchar", "1043", "0", "2200", "b", "S", "t", "-1", "f", "c", ",", "0", "NULL", "0", "0", "0", "1015"),
            ];

            for (typname, oid, typrelid, typnamespace, typtype, typcategory, typispreferred, typlen, typbyval, typalign, typdelim, typcollation, typdefault, typndims, typmodin, typmodout, typarray) in types_to_add {
                let mut encoder = TextDataRowEncoder::new(17);
                let _ = encoder.append_field(Some(typname));
                let _ = encoder.append_field(Some(oid));
                let _ = encoder.append_field(Some(typrelid));
                let _ = encoder.append_field(Some(typnamespace));
                let _ = encoder.append_field(Some(typtype));
                let _ = encoder.append_field(Some(typcategory));
                let _ = encoder.append_field(Some(typispreferred));
                let _ = encoder.append_field(Some(typlen));
                let _ = encoder.append_field(Some(typbyval));
                let _ = encoder.append_field(Some(typalign));
                let _ = encoder.append_field(Some(typdelim));
                let _ = encoder.append_field(Some(typcollation));
                let _ = encoder.append_field(Some(typdefault));
                let _ = encoder.append_field(Some(typndims));
                let _ = encoder.append_field(Some(typmodin));
                let _ = encoder.append_field(Some(typmodout));
                let _ = encoder.append_field(Some(typarray));
                response.result_set_mut().append_row(encoder.finish());
            }
            Some(response)
        } else if lower_query.contains("information_schema.tables") {
            let fields = vec![
                FieldInfo::new("table_schema".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("table_name".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("table_type".into(), None, None, Type::TEXT, FormatCode::Text),
            ];
            let response = Response::new(fields, Tag::new("SELECT 0"));
            Some(response)
        } else if lower_query.contains("information_schema.columns") {
            let fields = vec![
                FieldInfo::new("table_name".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("column_name".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("data_type".into(), None, None, Type::TEXT, FormatCode::Text),
                FieldInfo::new("is_nullable".into(), None, None, Type::TEXT, FormatCode::Text),
            ];
            let response = Response::new(fields, Tag::new("SELECT 0"));
            Some(response)
        } else if lower_query.contains("current_database()") || lower_query.contains("current_schema()") {
            let fields = vec![
                FieldInfo::new("result".into(), None, None, Type::TEXT, FormatCode::Text),
            ];
            let mut response = Response::new(fields, Tag::new("SELECT 1"));
            let mut encoder = TextDataRowEncoder::new(1);
            let value = if lower_query.contains("current_database") { "lumadb" } else { "public" };
            let _ = encoder.append_field(Some(value));
            response.result_set_mut().append_row(encoder.finish());
            Some(response)
        } else if lower_query.contains("version()") {
            let fields = vec![
                FieldInfo::new("version".into(), None, None, Type::TEXT, FormatCode::Text),
            ];
            let mut response = Response::new(fields, Tag::new("SELECT 1"));
            let mut encoder = TextDataRowEncoder::new(1);
            let _ = encoder.append_field(Some("LumaDB 3.0.0 (PostgreSQL compatible)"));
            response.result_set_mut().append_row(encoder.finish());
            Some(response)
        } else if lower_query.starts_with("set ") || lower_query.starts_with("show ") {
            // Handle SET and SHOW commands that Superset sends
            let fields = vec![];
            let response = Response::new(fields, Tag::new("OK"));
            Some(response)
        } else {
            None
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
        
        // Handle Superset/PostgreSQL system catalog queries
        if let Some(response) = self.handle_system_query(query) {
            return Ok(vec![response]);
        }
        
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
