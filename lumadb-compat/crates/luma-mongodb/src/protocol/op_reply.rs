use bytes::{BytesMut, Buf, BufMut};
use luma_protocol_core::{ProtocolError, Result};
use bson::Document;

pub struct OpReply {
    pub flags: i32,
    pub cursor_id: i64,
    pub starting_from: i32,
    pub number_returned: i32,
    pub documents: Vec<Document>,
}

impl OpReply {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 20 { return Err(ProtocolError::Protocol("Incomplete OpReply header".into())); }
        let flags = src.get_i32_le();
        let cursor_id = src.get_i64_le();
        let starting_from = src.get_i32_le();
        let number_returned = src.get_i32_le();
        
        let mut documents = Vec::with_capacity(number_returned as usize);
        for _ in 0..number_returned {
             if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OpReply doc size".into())); }
             let len_bytes = &src[0..4];
             let len = i32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
             if src.len() < len { return Err(ProtocolError::Protocol("Incomplete OpReply doc".into())); }
             
             let doc_slice = src.split_to(len);
             let doc = Document::from_reader(doc_slice.reader())
                .map_err(|e| ProtocolError::Protocol(format!("Invalid BSON in OpReply: {}", e)))?;
             documents.push(doc);
        }
        
        Ok(Self {
            flags,
            cursor_id,
            starting_from,
            number_returned,
            documents,
        })
    }
    
    pub fn write(&self, dst: &mut BytesMut) -> Result<()> {
        dst.put_i32_le(self.flags);
        dst.put_i64_le(self.cursor_id);
        dst.put_i32_le(self.starting_from);
        dst.put_i32_le(self.documents.len() as i32);
        
        for doc in &self.documents {
             let mut writer = Vec::new();
             doc.to_writer(&mut writer).map_err(|e| ProtocolError::Protocol(format!("BSON write error: {}", e)))?;
             dst.put_slice(&writer);
        }
        Ok(())
    }
}
