use std::io;

use serde_json::from_slice;
use serde_json::json;
use serde_json::to_vec_pretty;
use serde_json::Value;
use unnest::unnest_to_ndjson;

fn test_with(orig: &Value, expected: &[Value], target: usize) {
    let input = io::Cursor::new(to_vec_pretty(&orig).expect("serialisation of reference value"));
    let mut output = Vec::with_capacity(input.get_ref().len());
    let () = unnest_to_ndjson(input, &mut output, target).expect("unnest");
    let mut lines = Vec::with_capacity(expected.len());
    println!("{}", String::from_utf8_lossy(&output));
    for line in output.split(|&c| b'\n' == c) {
        if line.is_empty() {
            continue;
        }
        let line: Value = from_slice(line).expect("valid json");
        lines.push(line);
    }
    assert_eq!(expected, lines.as_slice());
}

#[test]
fn empty() {
    test_with(&json!({}), &[], 1);
    test_with(&json!([]), &[], 1);
}

#[test]
fn single_level_array() {
    test_with(
        &json!([
            5,
            { "foo": { "bar": 6, }, },
            "potato",
            true,
            { "baz": 6, },
        ]),
        &[
            json!({"key": [0], "value": 5, }),
            json!({"key": [1], "value": { "foo": { "bar": 6, }, }}),
            json!({"key": [2], "value": "potato", }),
            json!({"key": [3], "value": true, }),
            json!({"key": [4], "value": { "baz": 6, }, }),
        ],
        1,
    );
}
