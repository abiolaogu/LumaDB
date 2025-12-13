use bytes::{BytesMut, Buf, BufMut};
use luma_protocol_core::{ProtocolError, Result};
use bson::Document;

pub struct OpInsert {
    pub flags: i32,
    pub full_collection_name: String,
    pub documents: Vec<Document>,
}

pub struct OpUpdate {
    pub reserved: i32,
    pub full_collection_name: String,
    pub flags: i32,
    pub selector: Document,
    pub update: Document,
}

pub struct OpDelete {
    pub reserved: i32,
    pub full_collection_name: String,
    pub flags: i32,
    pub selector: Document,
}

impl OpInsert {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpInsert flags".into())); }
        let flags = src.get_i32_le();
        let full_collection_name = read_cstring(src)?;
        
        let mut documents = Vec::new();
        while src.has_remaining() {
            let doc = read_bson_document(src)?;
            documents.push(doc);
        }
        Ok(Self { flags, full_collection_name, documents })
    }
}

impl OpUpdate {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpUpdate reserved".into())); }
        let reserved = src.get_i32_le();
        let full_collection_name = read_cstring(src)?;
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpUpdate flags".into())); }
        let flags = src.get_i32_le();
        
        let selector = read_bson_document(src)?;
        let update = read_bson_document(src)?;
        
        Ok(Self { reserved, full_collection_name, flags, selector, update })
    }
}

impl OpDelete {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpDelete reserved".into())); }
        let reserved = src.get_i32_le();
        let full_collection_name = read_cstring(src)?;
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpDelete flags".into())); }
        let flags = src.get_i32_le();
        
        let selector = read_bson_document(src)?;
        
        Ok(Self { reserved, full_collection_name, flags, selector })
    }
}

// Helpers
fn read_cstring(src: &mut BytesMut) -> Result<String> {
    let mut bytes = Vec::new();
    let mut found_null = false;
    while src.has_remaining() {
        let b = src.get_u8();
        if b == 0 {
            found_null = true;
            break;
        }
        bytes.push(b);
    }
    if !found_null { return Err(ProtocolError::Protocol("Invalid CString".into())); }
    String::from_utf8(bytes).map_err(|_| ProtocolError::Protocol("Invalid UTF8 CString".into()))
}

fn read_bson_document(src: &mut BytesMut) -> Result<Document> {
    if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete BSON size".into())); }
    let len_bytes = &src[0..4];
    let len = i32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
    if src.len() < len { return Err(ProtocolError::Protocol("Incomplete BSON doc".into())); }
    let slice = src.split_to(len);
    Document::from_reader(slice.reader()).map_err(|e| ProtocolError::Protocol(format!("BSON error: {}", e)))
}
