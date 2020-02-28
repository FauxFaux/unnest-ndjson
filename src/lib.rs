use std::io;
use std::io::Bytes;
use std::io::Read;
use std::io::Write;
use std::iter::Peekable;

pub fn unnest_to_ndjson<R: Read, W: Write>(from: R, mut to: W, depth: usize) -> io::Result<()> {
    let mut iter = from.bytes().peekable();
    assert_eq!(depth, 0);
    drop_whitespace(&mut iter)?;
    scan_one(&mut iter, &mut to)?;
    Ok(())
}

fn drop_whitespace<R: Read>(from: &mut Peekable<Bytes<R>>) -> io::Result<()> {
    while let Some(Ok(b)) = from.peek() {
        if !b.is_ascii_whitespace() {
            return Ok(());
        }
        let _already_checked = from.next();
    }
    Ok(())
}

fn scan_one<R: Read, W: Write>(from: &mut Peekable<Bytes<R>>, into: &mut W) -> io::Result<()> {
    match from.next().ok_or(io::ErrorKind::UnexpectedEof)?? {
        b'{' => scan_object(from, into)?,
        b'[' => scan_array(from, into)?,
        b'"' => parse_string(from, into)?,
        c => scan_primitive(c, from, into)?,
    }
    Ok(())
}

fn scan_object<R: Read, W: Write>(from: &mut Peekable<Bytes<R>>, into: &mut W) -> io::Result<()> {
    into.write_all(b"{")?;
    loop {
        drop_whitespace(from)?;
        let s = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
        match s {
            b',' => continue,
            b'"' => (),
            b'}' => break,
            _ => return Err(io::ErrorKind::InvalidData.into()),
        }
        parse_string(from, into)?;
        drop_whitespace(from)?;
        let colon = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
        if b':' != colon {
            return Err(io::ErrorKind::InvalidData.into());
        }
        into.write_all(b":")?;
        drop_whitespace(from)?;
        scan_one(from, into)?;
        drop_whitespace(from)?;

        let delim = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
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

fn scan_array<R: Read, W: Write>(from: &mut Peekable<Bytes<R>>, into: &mut W) -> io::Result<()> {
    into.write_all(b"[")?;

    loop {
        drop_whitespace(from)?;
        if let Some(Ok(b']')) = from.peek() {
            let _infallible = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
            break;
        }

        scan_one(from, into)?;
        drop_whitespace(from)?;

        let delim = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
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
    from: &mut Peekable<Bytes<R>>,
    into: &mut W,
) -> io::Result<()> {
    into.write_all(&[start])?;
    while let Some(Ok(b)) = from.peek() {
        if b.is_ascii_whitespace()
            || b',' == *b
            || b']' == *b
            || b'}' == *b
            || b':' == *b
            || b.is_ascii_control()
        {
            break;
        }
        // infalliable, as we just peeked it
        let b = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
        into.write_all(&[b])?;
    }

    Ok(())
}

fn parse_string<R: Read, W: Write>(from: &mut Peekable<Bytes<R>>, into: &mut W) -> io::Result<()> {
    into.write_all(b"\"")?;
    loop {
        let b = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
        match b {
            b'"' => break,
            b'\r' | b'\n' => return Err(io::ErrorKind::InvalidData.into()),
            b'\\' => {
                let e = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
                match e {
                    b'"' | b'/' | b'\\' | b'b' | b'f' | b'r' | b'n' | b't' => (),
                    b'u' => {
                        for _ in 0..4 {
                            let h: u8 = from.next().ok_or(io::ErrorKind::UnexpectedEof)??;
                            if !h.is_ascii_hexdigit() {
                                return Err(io::ErrorKind::InvalidData.into());
                            }
                        }
                    }
                    b'\r' | b'\n' => return Err(io::ErrorKind::InvalidData.into()),
                    o => write!(into, "\\{}", o)?,
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
    use super::parse_string;
    use super::unnest_to_ndjson;
    use std::io;
    use std::io::Read;

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
        let mut buf = io::Cursor::new(buf.as_bytes()).bytes().peekable();
        // remove leading quote, as scan_one does
        buf.next().unwrap()?;
        parse_string(&mut buf, &mut v)?;
        Ok(String::from_utf8(v).unwrap())
    }

    #[test]
    fn string() -> io::Result<()> {
        assert_eq!(r#""hello world""#, ps(r#""hello world""#)?);
        Ok(())
    }
}
