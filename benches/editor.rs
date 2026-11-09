//! Reproducible microbenchmark, no external harness dependency.
use guinea_core::{
    Document, Movement, TextBuffer,
    coordinates::{CharIndex, cell_column},
};
use std::{
    hint::black_box,
    time::{Duration, Instant},
};

fn measure(mut operation: impl FnMut(), samples: usize) -> (f64, f64) {
    let mut durations = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        operation();
        durations.push(start.elapsed());
    }
    durations.sort();
    let ms = |value: Duration| value.as_secs_f64() * 1000.0;
    (
        ms(durations[samples / 2]),
        ms(durations[(samples * 95 / 100).min(samples - 1)]),
    )
}

fn report(size: usize, name: &str, operation: impl FnMut()) {
    let (p50, p95) = measure(operation, 21);
    println!("{size},{name},{p50:.3},{p95:.3}");
}

fn main() {
    println!("bytes,operation,p50_ms,p95_ms");
    let sizes =
        std::env::var("GUINEANOTE_BENCH_SIZES").unwrap_or_else(|_| "1024,1048576,10485760".into());
    let dir = tempfile::tempdir().unwrap();
    for size in sizes
        .split(',')
        .map(|s| s.parse::<usize>().expect("byte size"))
    {
        // Fixed ASCII line length plus Vietnamese, emoji, tab, CRLF coverage.
        let row = "fn main() { let tên = \"Việt 👩‍💻\"; }\t// benchmark\r\n";
        let mut text = row.repeat(size / row.len() + 1);
        while text.len() > size {
            text.pop();
        }
        let path = dir.path().join("bench.txt");
        std::fs::write(&path, &text).unwrap();
        report(size, "open_warm_cache", || {
            black_box(guinea_io::open(&path, false, u64::MAX).unwrap());
        });
        let mut document = guinea_io::open(&path, false, u64::MAX).unwrap();
        report(size, "save", || {
            guinea_io::save(&mut document, &path).unwrap();
        });
        let mut state: u64 = 0x12345678;
        report(size, "random_line_edit_undo_dirty", || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let line = state as usize % document.buffer().len_lines();
            let idx = document.buffer().line_to_char(line).unwrap();
            document.select(idx, idx).unwrap();
            document.insert("x").unwrap();
            black_box(document.is_dirty());
            document.undo().unwrap();
        });
        report(size, "scroll_100_lines", || {
            document.move_cursor(Movement::Start, false);
            for _ in 0..100 {
                document.move_cursor(Movement::Down, false);
                black_box(document.position());
            }
        });
        report(size, "search_absent", || {
            black_box(document.find("absent_needle"));
        });
        let long_text = "a".repeat(size);
        let mut long = Document::from_text(&long_text);
        report(size, "long_line_right_left_at_start", || {
            long.move_cursor(Movement::Start, false);
            long.move_cursor(Movement::Right, false);
            long.move_cursor(Movement::Left, false);
            black_box(long.position());
        });
        report(size, "long_line_cell_at_start", || {
            black_box(cell_column(&long_text, CharIndex(0), 4).unwrap());
        });
    }
}
