use bytes::{BytesMut};
use super::{CassandraProtocol, Frame, Opcode};
use super::frame::{StartupBody, write_string, read_string}; 
use super::messages::{RowsBody, RowsMetadata, ColSpec, ResultBody, ResultKind, QueryBody, QUERY_FLAG_VALUES, PrepareBody, ExecuteBody, BatchBody};
use crate::cql::types::CQLType;

// Mock read_long_string for writing manually
fn write_long_string(dst: &mut BytesMut, s: &str) {
    dst.extend_from_slice(&(s.len() as i32).to_be_bytes());
    dst.extend_from_slice(s.as_bytes());
}

#[test]
fn test_query_frame_with_options() {
    // Custom query frame with values
    // Opcode: Query (0x07)
    // Body: <long string query> + <consistency> + <flags> + <values count> + <value 1>...
    
    let mut body = BytesMut::new();
    write_long_string(&mut body, "SELECT * FROM users WHERE id = ?");
    body.extend_from_slice(&1u16.to_be_bytes()); // Consistency: ONE
    body.extend_from_slice(&QUERY_FLAG_VALUES.to_be_bytes()); // Flags: Values present
    
    // Values Count: 1
    body.extend_from_slice(&1u16.to_be_bytes());
    // Value 1: Int(123) -> Len 4 + [0, 0, 0, 123]
    body.extend_from_slice(&4i32.to_be_bytes());
    body.extend_from_slice(&123i32.to_be_bytes());

    let mut packet = BytesMut::new();
    packet.extend_from_slice(&[0x04, 0x00, 0x00, 0x01, 0x07]); // Header: v4, Id 1, Opcode Query
    packet.extend_from_slice(&(body.len() as u32).to_be_bytes());
    packet.extend_from_slice(&body);
    
    let frame = CassandraProtocol::read_frame(&mut packet).unwrap().expect("Should return frame");
    assert_eq!(frame.opcode, Opcode::Query);
    
    let mut f_body = frame.body;
    let query_msg = QueryBody::read(&mut f_body).unwrap();
    assert_eq!(query_msg.query, "SELECT * FROM users WHERE id = ?");
    assert_eq!(query_msg.options.consistency, 1);
    
    let values = query_msg.options.values.unwrap();
    assert_eq!(values.len(), 1);
    let val = values[0].as_ref().unwrap().as_ref().unwrap(); // Result<Option<Vec<u8>>>
    assert_eq!(val.len(), 4);
    assert_eq!(val[3], 123);
}

#[test]
fn test_prepare_frame() {
    let mut body = BytesMut::new();
    write_long_string(&mut body, "INSERT INTO foo (a) VALUES (?)");
    
    let mut packet = BytesMut::new();
    packet.extend_from_slice(&[0x04, 0x00, 0x00, 0x01, 0x09]); // PREPARE
    packet.extend_from_slice(&(body.len() as u32).to_be_bytes());
    packet.extend_from_slice(&body);

    let frame = CassandraProtocol::read_frame(&mut packet).unwrap().expect("Should return frame");
    assert_eq!(frame.opcode, Opcode::Prepare);

    let mut f_body = frame.body;
    let prep = PrepareBody::read(&mut f_body).unwrap();
    assert_eq!(prep.query, "INSERT INTO foo (a) VALUES (?)");
}

#[test]
fn test_result_rows_write() {
    let metadata = RowsMetadata {
        flags: 0x0001, 
        columns_count: 1,
        paging_state: None,
        col_specs: vec![ColSpec {
            ks_name: "ks".to_string(),
            table_name: "tb".to_string(),
            name: "col1".to_string(),
            type_: CQLType::Int,
        }],
    };

    let rows_content = vec![
        vec![Some(vec![0x00, 0x00, 0x00, 0x01])], // Row 1: 1
    ];

    let result = ResultBody {
        kind: ResultKind::Rows,
        rows: Some(RowsBody {
            metadata,
            rows_count: 1,
            rows_content,
        }),
    };

    let mut dst = BytesMut::new();
    result.write(&mut dst);
    assert!(dst.len() > 10);
}
