use crate::BUFFER_SIZE;

pub struct StackBuf {
    buf: [u8; BUFFER_SIZE],
    pos: usize,
}

impl StackBuf {
    pub fn new() -> StackBuf {
        StackBuf {
            buf: [0; BUFFER_SIZE],
            pos: 0,
        }
    }

    #[inline]
    pub fn write(&mut self, word: &[u8]) {
        self.buf[self.pos..self.pos + word.len()].copy_from_slice(word);
        self.pos += word.len();
    }

    #[inline]
    pub fn clear(&mut self) {
        self.pos = 0;
    }

    #[inline]
    pub fn getdata(&self) -> &[u8] {
        &self.buf[..self.pos]
    }

    #[inline]
    pub fn pos(&self) -> usize {
        self.pos
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.buf.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Default for StackBuf {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::StackBuf;
    #[test]
    fn test_stack_buf() {
        let buf = StackBuf::new();
        assert!(!buf.is_empty());

        let default_buf = StackBuf::default();
        assert_eq!(default_buf.pos, 0);
    }
}
