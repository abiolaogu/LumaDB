use bytes::{BytesMut, Buf, BufMut};
use luma_protocol_core::{ProtocolError, Result};
use bson::Document;

pub struct OpQuery {
    pub flags: u32,
    pub full_collection_name: String,
    pub number_to_skip: i32,
    pub number_to_return: i32,
    pub query: Document,
    pub return_fields_selector: Option<Document>,
}

impl OpQuery {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpQuery flags".into())); }
        let flags = src.get_u32_le();
        
        // Read CString name
        let mut name_bytes = Vec::new();
        let mut found_null = false;
        while src.has_remaining() {
            let b = src.get_u8();
            if b == 0 {
                found_null = true;
                break;
            }
            name_bytes.push(b);
        }
        if !found_null { return Err(ProtocolError::Protocol("Invalid CString in OpQuery name".into())); }
        let full_collection_name = String::from_utf8(name_bytes)
            .map_err(|_| ProtocolError::Protocol("Invalid UTF8 in OpQuery name".into()))?;
            
        if src.len() < 8 { return Err(ProtocolError::Protocol("Incomplete OpQuery params".into())); }
        let number_to_skip = src.get_i32_le();
        let number_to_return = src.get_i32_le();
        
        // Read BSON query
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpQuery doc size".into())); }
        let q_len_bytes = &src[0..4];
        let q_len = i32::from_le_bytes(q_len_bytes.try_into().unwrap()) as usize;
        if src.len() < q_len { return Err(ProtocolError::Protocol("Incomplete OpQuery document".into())); }
        
        let q_slice = src.split_to(q_len);
        let query = Document::from_reader(q_slice.reader())
             .map_err(|e| ProtocolError::Protocol(format!("Invalid BSON query: {}", e)))?;
             
        let mut return_fields_selector = None;
        if src.has_remaining() {
             if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete returnFieldsSelector size".into())); }
             let s_len_bytes = &src[0..4];
             let s_len = i32::from_le_bytes(s_len_bytes.try_into().unwrap()) as usize;
             if src.len() >= s_len {
                 let s_slice = src.split_to(s_len);
                 let doc = Document::from_reader(s_slice.reader())
                    .map_err(|e| ProtocolError::Protocol(format!("Invalid BSON selector: {}", e)))?;
                 return_fields_selector = Some(doc);
             }
        }
        
        Ok(Self {
            flags,
            full_collection_name,
            number_to_skip,
            number_to_return,
            query,
            return_fields_selector,
        })
    }
    
    pub fn write(&self, dst: &mut BytesMut) -> Result<()> {
        dst.put_u32_le(self.flags);
        dst.put_slice(self.full_collection_name.as_bytes());
        dst.put_u8(0); // CString null
        
        dst.put_i32_le(self.number_to_skip);
        dst.put_i32_le(self.number_to_return);
        
        let mut writer = Vec::new();
        self.query.to_writer(&mut writer).map_err(|e| ProtocolError::Protocol(format!("BSON write error: {}", e)))?;
        dst.put_slice(&writer);
        
        if let Some(doc) = &self.return_fields_selector {
            let mut writer = Vec::new();
            doc.to_writer(&mut writer).map_err(|e| ProtocolError::Protocol(format!("BSON write error: {}", e)))?;
            dst.put_slice(&writer);
        }
        Ok(())
    }
}
