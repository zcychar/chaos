#![allow(unused_imports)]

use std::cmp::min;

/// Looks like a file node, test-use only for now.
pub struct PseudoNode {
    pub content: Vec<u8>,
    pub ftype: u8,
}

impl PseudoNode {
    pub fn new(content: &str, file_type: u8) -> Self {
        Self {
            content: content.as_bytes().to_vec(),
            ftype: file_type,
        }
    }

    pub fn read_at(&self, offset: usize, buffer: &mut [u8]) -> usize {
        if offset >= self.content.len() {
            return 0;
        }
        let bytes_to_read = min(self.content.len() - offset, buffer.len());
        buffer[..bytes_to_read].copy_from_slice(&self.content[offset..offset + bytes_to_read]);
        bytes_to_read
    }

    pub fn write_at(&self, _offset: usize, _buffer: &[u8]) -> Result<usize, &'static str> {
        Err("nosup")
    }

    pub fn metadata_sz(&self) -> usize {
        self.content.len()
    }
}
