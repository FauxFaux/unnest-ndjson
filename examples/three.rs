use std::io;
use unnest::{unnest_to_ndjson, HeaderStyle};

fn main() -> io::Result<()> {
    let stdin = io::stdin();
    let stdin = stdin.lock();

    let stdout = io::stdout();
    let stdout = stdout.lock();

    unnest_to_ndjson(stdin, stdout, 3, HeaderStyle::None)
}
