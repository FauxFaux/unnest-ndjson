use std::io;
use unnest_ndjson::{unnest_to_ndjson, HeaderStyle};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdin = stdin.lock();

    let stdout = io::stdout();
    let stdout = stdout.lock();

    unnest_to_ndjson(stdin, stdout, 1, HeaderStyle::None)
}
