//! From https://github.com/khanghugo/dem/blob/master/src/byte_writer.rs

pub struct ByteWriter {
    pub data: Vec<u8>,
    offset: usize,
}

impl Default for ByteWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ByteWriter {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            offset: 0,
        }
    }

    fn offset(&mut self, offset: usize) {
        self.offset += offset;
    }

    pub fn get_offset(&self) -> usize {
        self.offset
    }

    pub fn append_u32(&mut self, i: u32) {
        self.data.extend(i.to_le_bytes());
        self.offset(4);
    }

    pub fn append_i32(&mut self, i: i32) {
        self.data.extend(i.to_le_bytes());
        self.offset(4);
    }

    pub fn append_u8(&mut self, i: u8) {
        self.data.extend(i.to_le_bytes());
        self.offset(1);
    }

    pub fn append_i8(&mut self, i: i8) {
        self.data.extend(i.to_le_bytes());
        self.offset(1);
    }

    pub fn append_i16(&mut self, i: i16) {
        self.data.extend(i.to_le_bytes());
        self.offset(2);
    }

    pub fn append_u8_slice(&mut self, i: &[u8]) {
        self.data.extend_from_slice(i);
        self.offset(i.len());
    }

    pub fn replace(&mut self, start: usize, length: usize, slice: &[u8]) {
        self.data[start..(length + start)].copy_from_slice(&slice[..length]);
    }

    pub fn replace_with_u32(&mut self, start: usize, val: u32) {
        let bytes = val.to_le_bytes();
        self.replace(start, 4, &bytes);
    }

    pub fn replace_with_i32(&mut self, start: usize, val: i32) {
        let bytes = val.to_le_bytes();
        self.replace(start, 4, &bytes);
    }

    // hopefully ascii :DDDDDDDDD
    pub fn append_string(&mut self, s: &str) {
        self.data.extend(s.as_bytes());
        // using len() directly on &str
        // if this is broken, blame rust
        self.offset(s.len())
    }

    pub fn append_f32(&mut self, i: f32) {
        self.data.extend(i.to_le_bytes());
        self.offset(4);
    }

    pub fn append_u16(&mut self, i: u16) {
        self.data.extend(i.to_le_bytes());
        self.offset(2);
    }
}
