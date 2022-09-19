use std::io::{self, Write};

pub trait MiniWrite {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
}

impl<T: Write> MiniWrite for T {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(self, buf)
    }
}

pub trait Sinker: MiniWrite {}

impl<T: Write> Sinker for T {}
