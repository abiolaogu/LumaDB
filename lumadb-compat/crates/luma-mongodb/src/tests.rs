use bytes::{BytesMut};
use crate::protocol::{OpMsg, MsgHeader, OpCode, OpQuery};
use crate::protocol::op_msg::{MsgFlags, Section};
use bson::doc;

#[test]
fn test_header_read_write() {
    let header = MsgHeader {
        message_length: 16, // minimal
        request_id: 1,
        response_to: 0,
        op_code: OpCode::OpMsg,
    };
    
    let mut buf = BytesMut::new();
    header.write(&mut buf);
    
    assert_eq!(buf.len(), 16);
    
    let decoded = MsgHeader::read(&mut buf).unwrap().expect("Should return header");
    assert_eq!(decoded.op_code, OpCode::OpMsg);
    assert_eq!(decoded.request_id, 1);
}

#[test]
fn test_op_msg_read_write() {
    let doc = doc! { "hello": 1, "isMaster": true };
    let section = Section::Body(doc.clone());
    
    let msg = OpMsg {
        flags: MsgFlags::empty(),
        sections: vec![section],
        checksum: None,
    };
    
    let mut buf = BytesMut::new();
    msg.write(&mut buf).unwrap();
    
    // Read back
    let decoded = OpMsg::read(&mut buf).unwrap();
    assert_eq!(decoded.sections.len(), 1);
    if let Section::Body(d) = &decoded.sections[0] {
        assert_eq!(d.get_i32("hello").unwrap(), 1);
    } else {
        panic!("Wrong section type");
    }
}

#[test]
fn test_op_query_read_write() {
    let query_doc = doc! { "find": "users" };
    let query = OpQuery {
        flags: 0,
        full_collection_name: "test.users".to_string(),
        number_to_skip: 0,
        number_to_return: 1,
        query: query_doc,
        return_fields_selector: None,
    };
    
    let mut buf = BytesMut::new();
    query.write(&mut buf).unwrap();
    
    let decoded = OpQuery::read(&mut buf).unwrap();
    assert_eq!(decoded.full_collection_name, "test.users");
}
