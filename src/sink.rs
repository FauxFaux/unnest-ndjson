use crate::HeaderStyle;
use std::io::{self, Write};

/// A simplification of the `Write` trait.
pub trait MiniWrite {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>;
}

impl<T: Write> MiniWrite for T {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        Write::write_all(self, buf)
    }
}

/// Consume the individual JSON documents.
///
/// For each document the following will be called, in this order:
///  * `observe_new_item`, with the path if it was computed
///  * `write_all` will be called repeatedly with the contents of the item
///  * `observe_end`, when the item is finished
///
/// The default implementation is to produce a stream of ndjson on an existing `Write` impl.
pub trait Sinker: MiniWrite {
    /// Called when a new item is started.
    ///
    /// `path` will be empty if it is not being computed.
    fn observe_new_item(&mut self, path: &[Vec<u8>], header_style: HeaderStyle) -> io::Result<()> {
        if header_style == HeaderStyle::None {
            return Ok(());
        }
        self.write_all(br#"{"key":["#)?;
        for (pos, path_segment) in path.iter().enumerate() {
            self.write_all(path_segment)?;
            if pos != path.len() - 1 {
                self.write_all(b",")?;
            }
        }
        self.write_all(br#"],"value":"#)?;
        Ok(())
    }

    /// Called when an item is finished.
    fn observe_end(&mut self, header_style: HeaderStyle) -> io::Result<()> {
        match header_style {
            HeaderStyle::None => self.write_all(b"\n"),
            HeaderStyle::PathArray => self.write_all(b"}\n"),
        }
    }
}

impl<T: Write> Sinker for T {}
