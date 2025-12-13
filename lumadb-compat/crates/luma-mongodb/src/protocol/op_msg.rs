use bytes::{Buf, BufMut, BytesMut};
use luma_protocol_core::{ProtocolError, Result};
use bson::Document;
use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct MsgFlags: u32 {
        const CHECKSUM_PRESENT = 1 << 0;
        const MORE_TO_COME = 1 << 1;
        const EXHAUST_ALLOWED = 1 << 16;
    }
}

#[derive(Debug)]
pub enum Section {
    Body(Document), // Kind 0
    DocSequence(String, Vec<Document>), // Kind 1: Identifier + list of documents
}

pub struct OpMsg {
    pub flags: MsgFlags,
    pub sections: Vec<Section>,
    pub checksum: Option<u32>,
}

impl OpMsg {
    pub fn read(src: &mut BytesMut) -> Result<Self> {
        if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete OP_MSG flags".into())); }
        let flags_bits = src.get_u32_le();
        let flags = MsgFlags::from_bits_truncate(flags_bits); // Use truncate to ignore unknown bits safely
        
        let mut sections = Vec::new();
        
        // Read sections until we hit checksum or end
        // Checksum is present if flag is set, it's the last 4 bytes of message
        
        // Note: The caller (MsgHeader::read) should have framed the 'src' buffer to exactly the message length minus header.
        // Wait, MsgHeader::read usually consumes header. src here is the BODY of the message.
        // So src length is Body Length.
        
        let has_checksum = flags.contains(MsgFlags::CHECKSUM_PRESENT);
        let end_offset = if has_checksum { 4 } else { 0 };
        
        while src.len() > end_offset {
             if src.len() < 1 { return Err(ProtocolError::Protocol("Incomplete section kind".into())); }
             let kind = src.get_u8();
             match kind {
                 0 => {
                     // Body: Single BSON Document
                     // BSON doc starts with int32 size
                     if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete section 0 size".into())); }
                     // We can peek size or let bson crate read it
                     // bson::Document::from_reader reads from a Reader. BytesMut impls Read.
                     // But we need to know if we have enough bytes. 
                     // Peek length
                     let len_bytes = &src[0..4];
                     let doc_len = i32::from_le_bytes(len_bytes.try_into().unwrap()) as usize;
                     if src.len() < doc_len { return Err(ProtocolError::Protocol("Incomplete BSON doc in section 0".into())); }
                     
                     let doc_slice = src.split_to(doc_len);
                     let doc = Document::from_reader(doc_slice.reader())
                        .map_err(|e| ProtocolError::Protocol(format!("Invalid BSON in section 0: {}", e)))?;
                     
                     sections.push(Section::Body(doc));
                 },
                 1 => {
                     // Document Sequence: Size (i32) + SeqId (CString) + List<BSON Document>
                     if src.len() < 4 { return Err(ProtocolError::Protocol("Incomplete section 1 size".into())); }
                     let seq_size = src.get_i32_le() as usize; 
                     // seq_size includes itself (4 bytes), identifier, and docs.
                     // src already advanced 4 bytes for size? No, get_i32_le advances.
                     // Wait, the size field *includes* the 4 bytes of size.
                     // So we consumed 4. Remaining in this sequence is seq_size - 4.
                     let remaining_seq = seq_size - 4;
                     if src.len() < remaining_seq { return Err(ProtocolError::Protocol("Incomplete document sequence".into())); }
                     
                     let mut seq_limit = src.take(remaining_seq); // Limit reader to this sequence
                     
                     // Read Identifier (CString)
                     // Since we work with BytesMut, we can't easily use CString::from_reader?
                     // We can read until null manually.
                     let mut id_bytes = Vec::new();
                     let mut found_null = false;
                     // Only scan a reasonable amount
                     let mut scanned = 0;
                     // We need to peek into seq_limit
                     // actually 'take' returns a Take<BytesMut>, simpler to just split_to if we knew length.
                     // But Identifier is variable length.
                     
                     // Alternative: Read seq_data into a separate buffer or handle logic on src directly
                     let mut seq_data = src.split_to(remaining_seq);
                     
                     while seq_data.has_remaining() {
                         let b = seq_data.get_u8();
                         if b == 0 {
                             found_null = true;
                             break;
                         }
                         id_bytes.push(b);
                         scanned += 1;
                         if scanned > 256 { return Err(ProtocolError::Protocol("Identifier too long".into())); }
                     }
                     
                     if !found_null { return Err(ProtocolError::Protocol("Invalid CString in section 1".into())); }
                     let identifier = String::from_utf8(id_bytes)
                        .map_err(|_| ProtocolError::Protocol("Invalid UTF8 in identifier".into()))?;
                        
                     let mut docs = Vec::new();
                     while seq_data.has_remaining() {
                          // Read BSON docs
                          if seq_data.len() < 4 { break; } // Padding or end? Sequence should be packed
                          let d_len_bytes = &seq_data[0..4];
                          let d_len = i32::from_le_bytes(d_len_bytes.try_into().unwrap()) as usize;
                          if seq_data.len() < d_len { return Err(ProtocolError::Protocol("Incomplete doc in sequence".into())); }
                          
                          let d_slice = seq_data.split_to(d_len);
                          let doc = Document::from_reader(d_slice.reader())
                             .map_err(|e| ProtocolError::Protocol(format!("Invalid BSON in sequence: {}", e)))?;
                          docs.push(doc);
                     }
                     sections.push(Section::DocSequence(identifier, docs));
                 },
                 _ => return Err(ProtocolError::Protocol(format!("Unknown section kind: {}", kind))),
             }
        }
        
        let checksum = if has_checksum {
            Some(src.get_u32_le())
        } else {
            None
        };
        
        Ok(Self { flags, sections, checksum })
    }
    
    pub fn write(&self, dst: &mut BytesMut) -> Result<()> {
        dst.put_u32_le(self.flags.bits());
        
        for section in &self.sections {
            match section {
                Section::Body(doc) => {
                    dst.put_u8(0);
                    // Write doc
                    let mut writer = Vec::new(); // Intermediate buffer for BSON
                    doc.to_writer(&mut writer).map_err(|e| ProtocolError::Protocol(format!("BSON write error: {}", e)))?;
                    dst.put_slice(&writer);
                },
                Section::DocSequence(id, docs) => {
                    dst.put_u8(1);
                    // We need total size (size 4 + id + 1 + docs)
                    // Calculate size first? Or reserve and patch?
                    // Patching is efficient.
                    let start_idx = dst.len();
                    dst.put_i32_le(0); // Placeholder
                    
                    dst.put_slice(id.as_bytes());
                    dst.put_u8(0); // CString null
                    
                    for doc in docs {
                         let mut writer = Vec::new();
                         doc.to_writer(&mut writer).map_err(|e| ProtocolError::Protocol(format!("BSON write error: {}", e)))?;
                         dst.put_slice(&writer);
                    }
                    
                    let end_idx = dst.len();
                    let total_len = (end_idx - start_idx) as i32;
                    // Patch size
                    let size_bytes = total_len.to_le_bytes();
                    // BytesMut is not easily indexable for mutable access like slice?
                    // It is: &mut dst[start_idx..start_idx+4]
                    dst[start_idx..start_idx+4].copy_from_slice(&size_bytes);
                }
            }
        }
        
        if let Some(crc) = self.checksum {
            dst.put_u32_le(crc);
        }
        Ok(())
    }
}
