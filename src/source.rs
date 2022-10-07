use std::io;
use std::io::Read;

use iowrap::ReadMany as _;

/// A more aggressive BufReader with some utility methods.
pub struct Source<R: Read> {
    inner: R,
    buf: [u8; 16 * 1024],
    len: usize,
    pos: usize,
}

impl<R: Read> Source<R> {
    pub fn new(inner: R) -> Self {
        Source {
            inner,
            buf: [0u8; 16 * 1024],
            len: 0,
            pos: 0,
        }
    }

    /// Attempt to read as much as possible into the buffer.
    ///
    /// If the buffer contains fully read data, discard it and fill the entire buffer again.
    ///
    /// Unlike BufReader, this will not give up the first time `read()` returns.
    pub fn fill(&mut self) -> io::Result<()> {
        if self.pos == self.len {
            self.pos = 0;
            self.len = 0;
        }
        let free = &mut self.buf[self.len..];
        let found = self.inner.read_many(free)?;
        if 0 == found && 0 == self.len {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }
        self.len += found;
        Ok(())
    }

    /// Access the valid portion of the buffer
    #[inline]
    pub fn buf(&self) -> &[u8] {
        &self.buf[self.pos..self.len]
    }

    /// Mark some amount of the `buf()` as consumed.
    #[inline]
    pub fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }

    /// Consume the entire buffer.
    ///
    /// This is more efficient than consume (although probably irrelevant in practice!).
    #[inline]
    pub fn all_useless(&mut self) {
        self.pos = 0;
        self.len = 0;
    }

    #[inline]
    pub fn next(&mut self) -> io::Result<u8> {
        loop {
            if self.pos < self.len {
                let ret = self.buf[self.pos];
                self.pos += 1;
                return Ok(ret);
            }
            self.fill()?;
        }
    }

    #[inline]
    pub fn peek(&mut self) -> io::Result<u8> {
        loop {
            if self.pos < self.len {
                return Ok(self.buf[self.pos]);
            }
            self.fill()?;
        }
    }
}
