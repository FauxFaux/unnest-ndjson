use std::io;
use std::io::Read;

use iowrap::ReadMany as _;

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

    pub fn buf(&self) -> &[u8] {
        &self.buf[self.pos..self.len]
    }

    pub fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }

    pub fn all_useless(&mut self) {
        self.pos = 0;
        self.len = 0;
    }

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

    pub fn peek(&mut self) -> io::Result<u8> {
        loop {
            if self.pos < self.len {
                return Ok(self.buf[self.pos]);
            }
            self.fill()?;
        }
    }
}
