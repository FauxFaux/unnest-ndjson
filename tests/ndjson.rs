use serde_json::from_slice;
use serde_json::json;
use serde_json::to_vec_pretty;
use serde_json::Value;
use std::io;
use std::io::Cursor;
use unnest_ndjson::{unnest_to_ndjson, HeaderStyle, MiniWrite, Sinker};

fn test_with(orig: &Value, expected: &[Value], target: usize, header_style: HeaderStyle) {
    let input = io::Cursor::new(to_vec_pretty(&orig).expect("serialisation of reference value"));
    let mut output = Vec::with_capacity(input.get_ref().len());
    let () = unnest_to_ndjson(input, &mut output, target, header_style).expect("unnest");
    let mut lines = Vec::with_capacity(expected.len());
    println!("{}", String::from_utf8_lossy(&output));
    for line in output.split(|&c| b'\n' == c) {
        if line.is_empty() {
            continue;
        }
        let line: Value = from_slice(line).expect("valid json");
        lines.push(line);
    }
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(
            expected.get(i),
            Some(line),
            "comparing line {} (zero indexed)",
            i
        );
    }
    assert_eq!(expected, lines.as_slice());
}

#[test]
fn empty() {
    test_with(&json!({}), &[], 1, HeaderStyle::PathArray);
    test_with(&json!([]), &[], 1, HeaderStyle::PathArray);
}

#[test]
fn single_level_array() {
    test_with(
        &json!([
            5,
            "potato",
            true,
            {},
            { "baz": 6, },
            { "foo": { "bar": 6, }, },
            { "aye": 7, "be": 8, },
            [],
            [ 5, ],
            [ 5, 6, ],
        ]),
        &[
            json!({"key": [0], "value": 5, }),
            json!({"key": [1], "value": "potato", }),
            json!({"key": [2], "value": true, }),
            json!({"key": [3], "value": {}, }),
            json!({"key": [4], "value": { "baz": 6, }, }),
            json!({"key": [5], "value": { "foo": { "bar": 6, }, }}),
            json!({"key": [6], "value": { "aye": 7, "be": 8, }, }),
            json!({"key": [7], "value": [], }),
            json!({"key": [8], "value": [ 5, ], }),
            json!({"key": [9], "value": [ 5, 6, ], }),
        ],
        1,
        HeaderStyle::PathArray,
    );
}

#[test]
fn single_level_object() {
    test_with(
        &json!({
            "number": 5,
            "string": "potato",
            "boolean": true,
            "emptyObject": {},
            "flatObject": { "baz": 6, },
            "nestedObject": { "foo": { "bar": 6, }, },
            "doubleObject": { "aye": 7, "be": 8, },
            "emptyArray": [],
            "singleArray": [ 5, ],
            "doubleArray": [ 5, 6, ],
        }),
        &[
            json!({"key": ["number"], "value": 5, }),
            json!({"key": ["string"], "value": "potato", }),
            json!({"key": ["boolean"], "value": true, }),
            json!({"key": ["emptyObject"], "value": {}, }),
            json!({"key": ["flatObject"], "value": { "baz": 6, }, }),
            json!({"key": ["nestedObject"], "value": { "foo": { "bar": 6, }, }, }),
            json!({"key": ["doubleObject"], "value": { "aye": 7, "be": 8, }, }),
            json!({"key": ["emptyArray"], "value": [], }),
            json!({"key": ["singleArray"], "value": [ 5, ], }),
            json!({"key": ["doubleArray"], "value": [ 5, 6, ], }),
        ],
        1,
        HeaderStyle::PathArray,
    );
}

#[test]
fn unicodes() {
    test_with(
        &json!({
            "bàh": 5,
            "five": "résumé",
        }),
        &[
            json!({"key": ["bàh"], "value": 5, }),
            json!({"key": ["five"], "value": "résumé", }),
        ],
        1,
        HeaderStyle::PathArray,
    );
}

#[derive(Default)]
struct Capture {
    caught: Vec<String>,
    current: Vec<u8>,
}

impl MiniWrite for &mut Capture {
    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.current.write_all(buf)
    }
}

impl Sinker for &mut Capture {
    fn observe_end(&mut self, _: HeaderStyle) -> io::Result<()> {
        self.caught
            .push(String::from_utf8(self.current.clone()).expect("valid utf8"));
        self.current.clear();
        Ok(())
    }
}

#[test]
fn unicodes_str() {
    let mut capture = Capture::default();
    unnest_to_ndjson(
        &br#"{"five": "r\u00ebr"}"#[..],
        &mut capture,
        1,
        HeaderStyle::None,
    )
    .expect("unnest");
    assert_eq!(capture.caught, vec!["\"r\\u00ebr\""]);
}

#[test]
fn double_level_object() {
    test_with(
        &json!({
            "number": 5,
            "string": "potato",
            "boolean": true,
            "emptyObject": {},
            "flatObject": { "baz": 6, },
            "nestedObject": { "foo": { "bar": 6, }, },
            "doubleObject": { "aye": 7, "be": 8, },
            "emptyArray": [],
            "singleArray": [ 5, ],
            "doubleArray": [ 5, 6, ],
        }),
        &[
            json!({"key": ["number"], "value": 5, }),
            json!({"key": ["string"], "value": "potato", }),
            json!({"key": ["boolean"], "value": true, }),
            json!({"key": ["flatObject", "baz"], "value": 6, }),
            json!({"key": ["nestedObject", "foo"], "value": { "bar": 6, }, }),
            json!({"key": ["doubleObject", "aye"], "value": 7, }),
            json!({"key": ["doubleObject", "be"], "value": 8, }),
            json!({"key": ["singleArray", 0], "value": 5, }),
            json!({"key": ["doubleArray", 0], "value": 5, }),
            json!({"key": ["doubleArray", 1], "value": 6, }),
        ],
        2,
        HeaderStyle::PathArray,
    );
}

#[test]
fn double_level_object_no_headers() {
    test_with(
        &json!({
            "number": 5,
            "string": "potato",
            "boolean": true,
            "emptyObject": {},
            "flatObject": { "baz": 6, },
            "nestedObject": { "foo": { "bar": 6, }, },
            "doubleObject": { "aye": 7, "be": 8, },
            "emptyArray": [],
            "singleArray": [ 5, ],
            "doubleArray": [ 5, 6, ],
        }),
        &[
            json!(5),
            json!("potato"),
            json!(true),
            json!(6),
            json!({ "bar": 6, }),
            json!(7),
            json!(8),
            json!(5),
            json!(5),
            json!(6),
        ],
        2,
        HeaderStyle::None,
    );
}
