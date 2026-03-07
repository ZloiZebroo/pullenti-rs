use flate2::read::GzDecoder;
use std::io::Read;

pub struct MorphDeserializer;

impl MorphDeserializer {
    pub fn deflate_gzip(data: &[u8]) -> Vec<u8> {
        let mut decoder = GzDecoder::new(data);
        let mut result = Vec::new();
        let _ = decoder.read_to_end(&mut result);
        result
    }
}
