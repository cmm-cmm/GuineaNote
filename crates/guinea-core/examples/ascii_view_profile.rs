//! Long-line viewport microbenchmark; not terminal input-to-flush.
use guinea_core::{Document, Movement, TextBuffer};
use std::{hint::black_box, time::Instant};

fn main() {
    const BYTES: usize = 10 * 1024 * 1024;
    println!("fixture,sample,operation,microseconds");
    for (fixture, unit) in [
        ("ascii", "abcd "),
        ("tabs", "ab\t"),
        ("unicode", "e\u{301} 界 👩‍💻 "),
    ] {
        let text = unit.repeat(BYTES / unit.len());
        let mut document = Document::from_text(&text);
        document.prepare_view(4);
        document.move_cursor(Movement::Finish, false);
        for sample in 0..200 {
            let started = Instant::now();
            let cell = document.cursor_cell(4);
            black_box(document.position());
            black_box(
                document
                    .line_window(0, cell.saturating_sub(80), 80, 4)
                    .unwrap(),
            );
            println!(
                "{fixture},{sample},end_viewport,{}",
                started.elapsed().as_micros()
            );
        }
        assert_eq!(document.buffer().len_chars(), text.chars().count());
        assert_eq!(document.text(), text);
    }
}
