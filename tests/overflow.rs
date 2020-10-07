use std::io;

use serde_json::from_slice;
use serde_json::json;
use serde_json::Value;
use unnest::unnest_to_ndjson;
use unnest::HeaderStyle;

#[test]
fn stack_abuse() {
    let mut buf = Vec::with_capacity(4000);
    let level = 8_634;
    // let level = 910;
    buf.extend_from_slice(&vec![b'['; level]);
    buf.push(b'5');
    buf.extend_from_slice(&vec![b']'; level]);
    let mut out = Vec::with_capacity(4);
    unnest_to_ndjson(
        &mut io::Cursor::new(buf),
        &mut out,
        level - 1,
        HeaderStyle::PathArray,
    )
    .expect("success");
    println!("{}", String::from_utf8_lossy(&out));
    let out: Value = from_slice(&out).expect("valid json");
    let out = out.as_object().expect("object");
    let key = out
        .get("key")
        .expect("key exists")
        .as_array()
        .expect("key array");
    let val = out
        .get("value")
        .expect("value exists")
        .as_array()
        .expect("out array");
    assert_eq!(level - 1, key.len());
    assert!(key.iter().all(|v| v == &json!(0)));
    assert_eq!(val, &vec![json!(5)]);
}
