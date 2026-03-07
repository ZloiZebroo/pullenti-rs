/// Binary deserializer for Pullenti .dat resource files
pub struct ByteArrayWrapper<'a> {
    data: &'a [u8],
}

impl<'a> ByteArrayWrapper<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        ByteArrayWrapper { data }
    }

    pub fn is_eof(&self, pos: usize) -> bool {
        pos >= self.data.len()
    }

    pub fn deserialize_byte(&self, pos: &mut usize) -> u8 {
        if *pos >= self.data.len() {
            return 0;
        }
        let b = self.data[*pos];
        *pos += 1;
        b
    }

    pub fn deserialize_short(&self, pos: &mut usize) -> i16 {
        if *pos + 1 >= self.data.len() {
            return 0;
        }
        let b0 = self.data[*pos] as i16;
        let b1 = self.data[*pos + 1] as i16;
        *pos += 2;
        (b1 << 8) | b0
    }

    pub fn deserialize_int(&self, pos: &mut usize) -> i32 {
        if *pos + 3 >= self.data.len() {
            return 0;
        }
        let b0 = self.data[*pos] as i32;
        let b1 = self.data[*pos + 1] as i32;
        let b2 = self.data[*pos + 2] as i32;
        let b3 = self.data[*pos + 3] as i32;
        *pos += 4;
        (b3 << 24) | (b2 << 16) | (b1 << 8) | b0
    }

    pub fn deserialize_string(&self, pos: &mut usize) -> String {
        if *pos >= self.data.len() {
            return String::new();
        }
        let len = self.data[*pos];
        *pos += 1;
        if len == 0xFF {
            return String::new();
        }
        if len == 0 {
            return String::new();
        }
        let len = len as usize;
        if *pos + len > self.data.len() {
            return String::new();
        }
        let s = String::from_utf8_lossy(&self.data[*pos..*pos + len]).to_string();
        *pos += len;
        s
    }

    pub fn deserialize_string_ex(&self, pos: &mut usize) -> String {
        if *pos >= self.data.len() {
            return String::new();
        }
        let len = self.deserialize_short(pos);
        if len == 0x7FFF || len < 0 {
            return String::new();
        }
        if len == 0 {
            return String::new();
        }
        let len = len as usize;
        if *pos + len > self.data.len() {
            return String::new();
        }
        let s = String::from_utf8_lossy(&self.data[*pos..*pos + len]).to_string();
        *pos += len;
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_int() {
        let data = [0x78, 0x56, 0x34, 0x12];
        let wrapper = ByteArrayWrapper::new(&data);
        let mut pos = 0;
        assert_eq!(wrapper.deserialize_int(&mut pos), 0x12345678);
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_deserialize_short() {
        let data = [0x34, 0x12];
        let wrapper = ByteArrayWrapper::new(&data);
        let mut pos = 0;
        assert_eq!(wrapper.deserialize_short(&mut pos), 0x1234);
    }

    #[test]
    fn test_deserialize_string() {
        let data = [3, b'a', b'b', b'c'];
        let wrapper = ByteArrayWrapper::new(&data);
        let mut pos = 0;
        assert_eq!(wrapper.deserialize_string(&mut pos), "abc");
    }
}
