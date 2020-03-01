use std::io;
use std::io::Read;
use std::io::Write;

use iowrap::ReadMany as _;

struct Source<R: Read> {
    inner: R,
    buf: [u8; 16 * 1024],
    len: usize,
    pos: usize,
}

impl<R: Read> Source<R> {
    fn new(inner: R) -> Self {
        Source {
            inner,
            buf: [0u8; 16 * 1024],
            len: 0,
            pos: 0,
        }
    }

    fn fill(&mut self) -> io::Result<()> {
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

    fn buf(&self) -> &[u8] {
        &self.buf[self.pos..self.len]
    }

    fn consume(&mut self, amt: usize) {
        self.pos += amt;
    }

    fn all_useless(&mut self) {
        self.pos = 0;
        self.len = 0;
    }

    fn next(&mut self) -> io::Result<u8> {
        loop {
            if self.pos < self.len {
                let ret = self.buf[self.pos];
                self.pos += 1;
                return Ok(ret);
            }
            self.fill()?;
        }
    }

    fn peek(&mut self) -> io::Result<u8> {
        loop {
            if self.pos < self.len {
                return Ok(self.buf[self.pos]);
            }
            self.fill()?;
        }
    }
}

pub fn unnest_to_ndjson<R: Read, W: Write>(from: R, mut to: W, depth: usize) -> io::Result<()> {
    let mut iter = Source::new(from);
    drop_whitespace(&mut iter)?;
    handle_one(&mut iter, &mut to, depth, &mut Vec::with_capacity(depth))?;
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

fn scan_one<R: Read, W: Write>(from: &mut Source<R>, into: &mut W) -> io::Result<()> {
    match from.next()? {
        b'{' => scan_object(from, into)?,
        b'[' => scan_array(from, into)?,
        b'"' => parse_string(from, into)?,
        c => scan_primitive(c, from, into)?,
    }
    Ok(())
}

fn handle_one<R: Read, W: Write>(
    from: &mut Source<R>,
    into: &mut W,
    depth: usize,
    path: &mut Vec<Vec<u8>>,
) -> io::Result<()> {
    if 0 == depth {
        write_prefix(into, path)?;
        scan_one(from, into)?;
        into.write_all(b"}\n")?;
        return Ok(());
    }
    match from.next()? {
        b'{' => handle_object(from, into, depth, path)?,
        b'[' => handle_array(from, into, depth, path)?,
        b'"' => {
            write_prefix(into, path)?;
            parse_string(from, into)?;
            into.write_all(b"}\n")?;
        }
        c => {
            write_prefix(into, path)?;
            scan_primitive(c, from, into)?;
            into.write_all(b"}\n")?;
        }
    }
    Ok(())
}

fn write_prefix<W: Write>(into: &mut W, path: &mut Vec<Vec<u8>>) -> io::Result<()> {
    into.write_all(br#"{"key":["#)?;
    for (pos, path_segment) in path.iter().enumerate() {
        into.write_all(path_segment)?;
        if pos != path.len() - 1 {
            into.write_all(b",")?;
        }
    }
    into.write_all(br#"],"value":"#)?;
    Ok(())
}

fn handle_object<R: Read, W: Write>(
    from: &mut Source<R>,
    into: &mut W,
    depth: usize,
    path: &mut Vec<Vec<u8>>,
) -> io::Result<()> {
    loop {
        drop_whitespace(from)?;
        let s = from.next()?;
        match s {
            b',' => continue,
            b'"' => (),
            b'}' => break,
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        {
            let mut key = Vec::with_capacity(32);
            parse_string(from, &mut key)?;
            path.push(key);
        }
        drop_whitespace(from)?;
        let colon = from.next()?;
        if b':' != colon {
            return Err(io::ErrorKind::InvalidData.into());
        }
        drop_whitespace(from)?;
        handle_one(from, into, depth - 1, path)?;
        drop_whitespace(from)?;

        let _ = path.pop().unwrap();

        let delim = from.next()?;
        match delim {
            b'}' => break,
            b',' => (),
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
    }
    Ok(())
}

fn handle_array<R: Read, W: Write>(
    from: &mut Source<R>,
    into: &mut W,
    depth: usize,
    path: &mut Vec<Vec<u8>>,
) -> io::Result<()> {
    for idx in 0usize.. {
        drop_whitespace(from)?;
        if let Ok(b']') = from.peek() {
            let _infallible = from.next()?;
            break;
        }

        path.push(format!("{}", idx).into_bytes());
        handle_one(from, into, depth - 1, path)?;
        let _ = path.pop().unwrap();
        drop_whitespace(from)?;

        let delim = from.next()?;
        match delim {
            b']' => break,
            b',' => (),
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
    }
    Ok(())
}

fn scan_object<R: Read, W: Write>(from: &mut Source<R>, into: &mut W) -> io::Result<()> {
    into.write_all(b"{")?;
    loop {
        drop_whitespace(from)?;
        let s = from.next()?;
        match s {
            b',' => continue,
            b'"' => (),
            b'}' => break,
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        parse_string(from, into)?;
        drop_whitespace(from)?;
        let colon = from.next()?;
        if b':' != colon {
            return Err(io::ErrorKind::InvalidData.into());
        }
        into.write_all(b":")?;
        drop_whitespace(from)?;
        scan_one(from, into)?;
        drop_whitespace(from)?;

        let delim = from.next()?;
        match delim {
            b'}' => break,
            b',' => (),
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        into.write_all(b",")?;
    }
    into.write_all(b"}")?;
    Ok(())
}

fn scan_array<R: Read, W: Write>(from: &mut Source<R>, into: &mut W) -> io::Result<()> {
    into.write_all(b"[")?;

    loop {
        drop_whitespace(from)?;
        if let Ok(b']') = from.peek() {
            let _infallible = from.next()?;
            break;
        }

        scan_one(from, into)?;
        drop_whitespace(from)?;

        let delim = from.next()?;
        match delim {
            b']' => break,
            b',' => (),
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        into.write_all(b",")?;
    }
    into.write_all(b"]")?;
    Ok(())
}

fn scan_primitive<R: Read, W: Write>(
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

fn parse_string<R: Read, W: Write>(from: &mut Source<R>, into: &mut W) -> io::Result<()> {
    into.write_all(b"\"")?;
    loop {
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
    use super::unnest_to_ndjson;
    use super::Source;

    #[test]
    fn zero_depth() -> io::Result<()> {
        let mut dest = Vec::with_capacity(128);
        unnest_to_ndjson(io::Cursor::new(br#" { "foo" : -5e6 } "#), &mut dest, 0)?;
        assert_eq!(
            String::from_utf8_lossy(br#"{"foo":-5e6}"#),
            String::from_utf8_lossy(dest.as_slice())
        );
        Ok(())
    }

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
