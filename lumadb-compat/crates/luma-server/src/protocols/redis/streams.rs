//! Redis Streams Commands
//! XADD, XLEN, XRANGE, XREVRANGE, XREAD

use super::{RedisStore, RedisValue, RespValue, StreamData, StreamEntry};

/// Execute stream commands
pub fn execute_stream_command(
    cmd: &str,
    args: &[String],
    store: &RedisStore,
) -> Option<RespValue> {
    match cmd {
        "XADD" => Some(xadd(args, store)),
        "XLEN" => Some(xlen(args, store)),
        "XRANGE" => Some(xrange(args, store)),
        "XREVRANGE" => Some(xrevrange(args, store)),
        "XREAD" => Some(xread(args, store)),
        _ => None,
    }
}

fn xadd(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 4 {
        return RespValue::Error("ERR wrong number of arguments for 'xadd' command".to_string());
    }
    
    let key = &args[1];
    let id = &args[2];
    
    let mut fields = std::collections::HashMap::new();
    let mut i = 3;
    while i + 1 < args.len() {
        fields.insert(args[i].clone(), args[i + 1].clone());
        i += 2;
    }
    
    match store.xadd(key, if id == "*" { None } else { Some(id) }, fields) {
        Some(entry_id) => RespValue::BulkString(Some(entry_id)),
        None => RespValue::Error("ERR failed to add entry".to_string()),
    }
}

fn xlen(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments".to_string());
    }
    RespValue::Integer(store.xlen(&args[1]) as i64)
}

fn xrange(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 4 {
        return RespValue::Error("ERR wrong number of arguments".to_string());
    }
    
    let key = &args[1];
    let start = &args[2];
    let end = &args[3];
    let count = args.get(5).and_then(|s| s.parse().ok());
    
    let entries = store.xrange(key, start, end, count);
    entries_to_resp(&entries)
}

fn xrevrange(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 4 {
        return RespValue::Error("ERR wrong number of arguments".to_string());
    }
    
    let key = &args[1];
    let end = &args[2];
    let start = &args[3];
    let count = args.get(5).and_then(|s| s.parse().ok());
    
    let entries = store.xrevrange(key, start, end, count);
    entries_to_resp(&entries)
}

fn xread(args: &[String], store: &RedisStore) -> RespValue {
    // XREAD [COUNT count] [BLOCK ms] STREAMS key [key ...] id [id ...]
    let mut i = 1;
    let mut count: Option<usize> = None;
    let mut _block_ms: Option<u64> = None;
    
    while i < args.len() {
        match args[i].to_uppercase().as_str() {
            "COUNT" => {
                count = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "BLOCK" => {
                _block_ms = args.get(i + 1).and_then(|s| s.parse().ok());
                i += 2;
            }
            "STREAMS" => {
                i += 1;
                break;
            }
            _ => i += 1,
        }
    }
    
    // Find keys and IDs
    let remaining: Vec<_> = args[i..].to_vec();
    let half = remaining.len() / 2;
    let keys = &remaining[..half];
    let ids = &remaining[half..];
    
    let mut results = Vec::new();
    for (j, key) in keys.iter().enumerate() {
        let id = ids.get(j).map(|s| s.as_str()).unwrap_or("0-0");
        let entries = store.xread(key, id, count);
        if !entries.is_empty() {
            results.push(RespValue::Array(Some(vec![
                RespValue::BulkString(Some(key.clone())),
                entries_to_resp(&entries),
            ])));
        }
    }
    
    if results.is_empty() {
        RespValue::Null
    } else {
        RespValue::Array(Some(results))
    }
}

fn entries_to_resp(entries: &[StreamEntry]) -> RespValue {
    let items: Vec<RespValue> = entries.iter().map(|e| {
        let fields: Vec<RespValue> = e.fields.iter()
            .flat_map(|(k, v)| vec![
                RespValue::BulkString(Some(k.clone())),
                RespValue::BulkString(Some(v.clone())),
            ])
            .collect();
        RespValue::Array(Some(vec![
            RespValue::BulkString(Some(e.id.clone())),
            RespValue::Array(Some(fields)),
        ]))
    }).collect();
    RespValue::Array(Some(items))
}
