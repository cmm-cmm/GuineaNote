//! Local core timings, separate from terminal input-to-flush.
use guinea_core::{Document, Movement};
use std::{hint::black_box, time::Instant};

fn main() {
    let unit = "a e\u{301} 界 👩‍💻 ";
    let text = format!(
        "marker {}",
        unit.repeat((10 * 1024 * 1024 - 7) / unit.len())
    );
    let mut document = Document::from_text(&text);
    document.prepare_view(4);
    println!("iteration,operation,microseconds");
    for iteration in 0..1000 {
        for operation in ["insert", "delete", "undo", "redo"] {
            let start = Instant::now();
            match operation {
                "insert" => document.insert("x").unwrap(),
                "delete" => document.backspace().unwrap(),
                "undo" => {
                    document.undo().unwrap();
                }
                "redo" => {
                    document.redo().unwrap();
                }
                _ => unreachable!(),
            }
            let elapsed = start.elapsed().as_micros();
            black_box(document.position());
            println!("{iteration},{operation},{elapsed}");
        }
        document.move_cursor(Movement::Start, false);
    }
    assert_eq!(document.text(), text);
    let mut contiguous = text.clone();
    for iteration in 0..200 {
        let start = Instant::now();
        contiguous.insert(0, 'x');
        println!("{iteration},string_insert,{}", start.elapsed().as_micros());
        let start = Instant::now();
        contiguous.replace_range(0..1, "");
        println!("{iteration},string_delete,{}", start.elapsed().as_micros());
    }
    assert_eq!(contiguous, text);
}
