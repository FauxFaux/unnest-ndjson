use itertools::Itertools;
use maplit::{convert_args, hashmap};
use std::collections::HashMap;
use std::io;
use unnest_ndjson::{unnest_to_ndjson, HeaderStyle, MiniWrite, Sinker};

#[test]
fn load_map() -> io::Result<()> {
    #[derive(Default)]
    struct Capture {
        inner: HashMap<String, String>,
        key: String,
        current: Vec<u8>,
    }

    impl MiniWrite for &mut Capture {
        fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
            self.current.write_all(buf)
        }
    }

    impl Sinker for &mut Capture {
        fn observe_new_item(&mut self, path: &[Vec<u8>], _: HeaderStyle) -> io::Result<()> {
            self.key = path
                .iter()
                .map(|b| String::from_utf8_lossy(b).to_string())
                .join(" // ");
            Ok(())
        }

        fn observe_end(&mut self, _: HeaderStyle) -> io::Result<()> {
            let value = String::from_utf8_lossy(&self.current).to_string();
            self.current.truncate(0);
            self.inner.insert(self.key.clone(), value);
            Ok(())
        }
    }

    let mut map = Capture::default();
    unnest_to_ndjson(
        io::Cursor::new(br#"["a", "b", "c"]"#),
        &mut map,
        1,
        HeaderStyle::PathArray,
    )?;

    assert_eq!(
        map.inner,
        convert_args!(hashmap!(
            "0" => r#""a""#,
            "1" => r#""b""#,
            "2" => r#""c""#,
        )),
    );

    Ok(())
}
