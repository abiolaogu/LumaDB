use bytes::{BufMut, BytesMut};
use super::frame::write_string;

pub struct SaslStart {
    pub mechanism: String,
    pub data: Option<Vec<u8>>,
}

impl SaslStart {
    pub fn write(&self, dst: &mut BytesMut) {
        write_string(dst, &self.mechanism);
        match &self.data {
            Some(d) => {
                dst.put_i32(d.len() as i32);
                dst.put_slice(d);
            },
            None => dst.put_i32(-1),
        }
    }
}
