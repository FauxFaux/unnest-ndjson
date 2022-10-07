//! Convert a large json document into smaller, easier to process documents, quickly.
//!
//! Call [unnest_to_ndjson] on your stream, and receive a much nicer stream, or some
//! callbacks.

use std::convert::TryFrom;
use std::io;
use std::io::Read;

use iowrap::Ignore;
use memchr::memchr;

mod sink;
mod source;

pub use crate::sink::{MiniWrite, Sinker};
use source::Source;

/// Control what information is retained for individual result documents
#[derive(Copy, Clone, Eq, PartialEq)]
#[non_exhaustive]
pub enum HeaderStyle {
    /// No information is retained.
    None,
    /// The path to the child document is retained.
    ///
    /// `{"a": {"H": 6}, "b": {"H": 7}}` would become,
    /// with the default formatter and a target of `1`,
    /// `{"key":["a"],"value":{"H":6}}` and
    /// `{"key":["b"],"value":{"H":6}}`
    PathArray,
}

struct Loc {
    depth: isize,
    path: Vec<Vec<u8>>,
    header_style: HeaderStyle,
}

impl Loc {
    fn at_target(&self) -> bool {
        0 == self.depth
    }

    fn collecting_keys(&self) -> bool {
        self.depth <= 0
    }

    fn producing_regular_output(&self) -> bool {
        self.depth > 0
    }

    fn shallower_than_target(&self) -> bool {
        self.depth < 0
    }

    fn write_suffix(&self, into: &mut impl Sinker) -> io::Result<()> {
        into.observe_end(self.header_style)
    }

    fn compute_header(&self) -> bool {
        match self.header_style {
            HeaderStyle::None => false,
            HeaderStyle::PathArray => true,
        }
    }
}

/// Consume a large JSON document from a `Read`, and write sub documents to a destination.
///
/// The typical destination is just a `Write` implementation, like a [std::fs::File],
/// or a [Vec]. Alternatively, you can use the [Sinker] interface to get access to fragments
/// of documents.
///
/// Configure the level of un-nesting with the `target` parameter. `1` will remove one level
/// of nesting, such as converting `[{"a":5}, {"a":6}]` into `{"a":5}` and `{"a":6}`.
///
/// `header_style` controls how much context to retain. See [HeaderStyle].
pub fn unnest_to_ndjson<R: Read>(
    from: R,
    mut to: impl Sinker,
    target: usize,
    header_style: HeaderStyle,
) -> io::Result<()> {
    let mut iter = Source::new(from);
    let depth = -isize::try_from(target).map_err(|_| io::ErrorKind::InvalidData)?;
    let mut loc = Loc {
        depth,
        path: Vec::with_capacity(target),
        header_style,
    };
    loop {
        match drop_whitespace(&mut iter) {
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(e) => Err(e)?,
            Ok(()) => (),
        }
        handle_one(&mut iter, &mut to, &mut loc)?;
    }
    Ok(())
}

fn drop_whitespace<R: Read>(from: &mut Source<R>) -> io::Result<()> {
    loop {
        match from.buf().iter().position(|&b| !b.is_ascii_whitespace()) {
            Some(end) => {
                from.consume(end);
                return Ok(());
            }
            None => {
                from.all_useless();
                from.fill()?;
            }
        }
    }
}

fn handle_one<R: Read>(
    from: &mut Source<R>,
    into: &mut impl Sinker,
    loc: &mut Loc,
) -> io::Result<()> {
    if loc.compute_header() && loc.at_target() {
        into.observe_new_item(&loc.path, loc.header_style)?;
    }
    match from.next()? {
        b'{' => handle_object(from, into, loc)?,
        b'[' => handle_array(from, into, loc)?,
        c => {
            if loc.compute_header() && loc.shallower_than_target() {
                into.observe_new_item(&loc.path, loc.header_style)?;
            }
            if b'"' == c {
                parse_string(from, into)?;
            } else {
                scan_primitive(c, from, into)?
            }
            if loc.shallower_than_target() {
                loc.write_suffix(into)?;
            }
        }
    }
    if loc.at_target() {
        loc.write_suffix(into)?;
    }
    Ok(())
}

fn handle_object<R: Read>(
    from: &mut Source<R>,
    into: &mut impl Sinker,
    loc: &mut Loc,
) -> io::Result<()> {
    loc.depth += 1;

    if loc.producing_regular_output() {
        into.write_all(b"{")?;
    }
    loop {
        drop_whitespace(from)?;
        let s = from.next()?;
        match s {
            b',' => continue,
            b'"' => (),
            b'}' => break,
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        if loc.producing_regular_output() {
            parse_string(from, into)?;
        } else {
            assert!(loc.collecting_keys());
            if loc.compute_header() {
                let mut key = Vec::with_capacity(32);
                parse_string(from, &mut key)?;
                loc.path.push(key);
            } else {
                parse_string(from, &mut Ignore {})?;
            }
        }
        drop_whitespace(from)?;
        let colon = from.next()?;
        if b':' != colon {
            return Err(io::ErrorKind::InvalidData.into());
        }
        if loc.producing_regular_output() {
            into.write_all(b":")?;
        }
        drop_whitespace(from)?;
        handle_one(from, into, loc)?;
        drop_whitespace(from)?;

        if loc.compute_header() && loc.collecting_keys() {
            let _ = loc.path.pop().unwrap();
        }

        let delim = from.next()?;
        match delim {
            b'}' => break,
            b',' => (),
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        if loc.producing_regular_output() {
            into.write_all(b",")?;
        }
    }
    if loc.producing_regular_output() {
        into.write_all(b"}")?;
    }

    loc.depth -= 1;

    Ok(())
}

fn handle_array<R: Read>(
    from: &mut Source<R>,
    into: &mut impl Sinker,
    loc: &mut Loc,
) -> io::Result<()> {
    loc.depth += 1;

    if loc.producing_regular_output() {
        into.write_all(b"[")?;
    }

    for idx in 0usize.. {
        drop_whitespace(from)?;
        if let Ok(b']') = from.peek() {
            let _infallible = from.next()?;
            break;
        }

        if loc.compute_header() && loc.collecting_keys() {
            loc.path.push(format!("{}", idx).into_bytes());
        }
        handle_one(from, into, loc)?;
        if loc.compute_header() && loc.collecting_keys() {
            let _ = loc.path.pop().unwrap();
        }

        drop_whitespace(from)?;

        let delim = from.next()?;
        match delim {
            b']' => break,
            b',' => (),
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        if loc.producing_regular_output() {
            into.write_all(b",")?;
        }
    }
    if loc.producing_regular_output() {
        into.write_all(b"]")?;
    }

    loc.depth -= 1;

    Ok(())
}

fn scan_primitive<R: Read, W: sink::MiniWrite>(
    start: u8,
    from: &mut Source<R>,
    into: &mut W,
) -> io::Result<()> {
    into.write_all(&[start])?;
    while let Ok(b) = from.peek() {
        if b.is_ascii_whitespace()
            || b',' == b
            || b']' == b
            || b'}' == b
            || b':' == b
            || b.is_ascii_control()
        {
            break;
        }
        // infalliable, as we just peeked it
        let b = from.next()?;
        into.write_all(&[b])?;
    }

    Ok(())
}

fn parse_string<R: Read, W: sink::MiniWrite>(from: &mut Source<R>, into: &mut W) -> io::Result<()> {
    into.write_all(b"\"")?;
    loop {
        let buf = from.buf();
        let quote = memchr(b'"', buf).unwrap_or(buf.len());
        let escape = memchr(b'\\', buf).unwrap_or(buf.len());
        let safe = quote.min(escape);
        into.write_all(&buf[..safe])?;
        from.consume(safe);
        let b = from.next()?;
        match b {
            b'"' => break,
            b'\r' | b'\n' => return Err(io::ErrorKind::InvalidData.into()),
            b'\\' => {
                let e = from.next()?;
                match e {
                    b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => {
                        into.write_all(&[b'\\', e])?;
                    }
                    b'u' => {
                        for _ in 0..4 {
                            let h: u8 = from.next()?;
                            if !h.is_ascii_hexdigit() {
                                return Err(io::ErrorKind::InvalidData.into());
                            }
                        }
                    }
                    _ => return Err(io::ErrorKind::InvalidData.into()),
                }
            }
            o => into.write_all(&[o])?,
        }
    }
    into.write_all(b"\"")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io;

    use super::parse_string;
    use super::Source;

    fn ps(buf: &str) -> io::Result<String> {
        let mut v = Vec::with_capacity(buf.len());
        let mut buf = Source::new(io::Cursor::new(buf.as_bytes()));
        // remove leading quote, as scan_one does
        buf.next()?;
        parse_string(&mut buf, &mut v)?;
        Ok(String::from_utf8(v).unwrap())
    }

    #[test]
    fn string() -> io::Result<()> {
        assert_eq!(r#""hello world""#, ps(r#""hello world""#)?);
        Ok(())
    }
}
