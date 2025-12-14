//! Redis Pub/Sub Commands
//! PUBLISH, SUBSCRIBE, PSUBSCRIBE, UNSUBSCRIBE, PUNSUBSCRIBE

use super::{RedisStore, RespValue};
use tokio::sync::broadcast;

/// Execute pub/sub commands
pub fn execute_pubsub_command(
    cmd: &str,
    args: &[String],
    store: &RedisStore,
) -> Option<RespValue> {
    match cmd {
        "PUBLISH" => Some(publish(args, store)),
        "SUBSCRIBE" => Some(subscribe(args, store)),
        "PSUBSCRIBE" => Some(psubscribe(args, store)),
        "UNSUBSCRIBE" => Some(unsubscribe(args)),
        "PUNSUBSCRIBE" => Some(punsubscribe(args)),
        _ => None,
    }
}

fn publish(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 3 {
        return RespValue::Error("ERR wrong number of arguments".to_string());
    }
    let subscribers = store.publish(&args[1], args[2].clone());
    RespValue::Integer(subscribers)
}

fn subscribe(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments".to_string());
    }
    
    let mut responses = Vec::new();
    for (i, channel) in args[1..].iter().enumerate() {
        let _rx = store.subscribe(channel);
        responses.push(RespValue::Array(Some(vec![
            RespValue::BulkString(Some("subscribe".to_string())),
            RespValue::BulkString(Some(channel.clone())),
            RespValue::Integer((i + 1) as i64),
        ])));
    }
    
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespValue::Array(Some(responses))
    }
}

fn psubscribe(args: &[String], store: &RedisStore) -> RespValue {
    if args.len() < 2 {
        return RespValue::Error("ERR wrong number of arguments".to_string());
    }
    
    let mut responses = Vec::new();
    for (i, pattern) in args[1..].iter().enumerate() {
        let _rx = store.subscribe(pattern);
        responses.push(RespValue::Array(Some(vec![
            RespValue::BulkString(Some("psubscribe".to_string())),
            RespValue::BulkString(Some(pattern.clone())),
            RespValue::Integer((i + 1) as i64),
        ])));
    }
    
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespValue::Array(Some(responses))
    }
}

fn unsubscribe(args: &[String]) -> RespValue {
    let channels = if args.len() < 2 {
        vec!["*".to_string()]
    } else {
        args[1..].to_vec()
    };
    
    let mut responses = Vec::new();
    for (i, channel) in channels.iter().enumerate() {
        responses.push(RespValue::Array(Some(vec![
            RespValue::BulkString(Some("unsubscribe".to_string())),
            RespValue::BulkString(Some(channel.clone())),
            RespValue::Integer((channels.len() - i - 1) as i64),
        ])));
    }
    
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespValue::Array(Some(responses))
    }
}

fn punsubscribe(args: &[String]) -> RespValue {
    let patterns = if args.len() < 2 {
        vec!["*".to_string()]
    } else {
        args[1..].to_vec()
    };
    
    let mut responses = Vec::new();
    for (i, pattern) in patterns.iter().enumerate() {
        responses.push(RespValue::Array(Some(vec![
            RespValue::BulkString(Some("punsubscribe".to_string())),
            RespValue::BulkString(Some(pattern.clone())),
            RespValue::Integer((patterns.len() - i - 1) as i64),
        ])));
    }
    
    if responses.len() == 1 {
        responses.remove(0)
    } else {
        RespValue::Array(Some(responses))
    }
}
