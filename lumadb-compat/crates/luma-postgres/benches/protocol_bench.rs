use criterion::{black_box, criterion_group, criterion_main, Criterion};
use bytes::{BytesMut, BufMut};
use luma_postgres::protocol::StartupMessage;
use luma_postgres::types::{encode_value, FORMAT_TEXT};
use luma_protocol_core::Value;

pub fn benchmark_startup_message_parse(c: &mut Criterion) {
    let mut buf = BytesMut::new();
    // Pre-allocate efficient buffer for benchmark
    let mut body = BytesMut::new();
    body.put_i32(196608); 
    body.put_slice(b"user\0postgres\0\0");
    let len = 4 + body.len() as i32;
    buf.put_i32(len);
    buf.put(body);
    let data = buf.freeze(); // Immutable source

    c.bench_function("parse_startup_message", |b| {
        b.iter(|| {
            // We need mutable buffer, so we clone the data each time?
            // Parsing consumes the buffer.
            let mut b = BytesMut::from(data.as_ref());
            StartupMessage::parse(black_box(&mut b)).unwrap()
        })
    });
}

pub fn benchmark_encode_value(c: &mut Criterion) {
    let val = Value::Int32(123456);
    c.bench_function("encode_int32_text", |b| {
        b.iter(|| {
            encode_value(black_box(&val), FORMAT_TEXT).unwrap()
        })
    });
}

criterion_group!(benches, benchmark_startup_message_parse, benchmark_encode_value);
criterion_main!(benches);
