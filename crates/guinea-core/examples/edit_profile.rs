//! Core timings of whole-document and multi-line edit commands on a ~10 MiB
//! multi-line source corpus (Unicode, CRLF). These run on the input thread
//! today unless noted, so each one is a frame the user waits for.
//!     cargo run --release -p guinea-core --example edit_profile
use guinea_core::{CaseConversion, Document, LineOperation, Movement, SearchOptions, TextBuffer};
use std::time::Instant;

fn corpus() -> String {
    let mut text = String::new();
    let mut n = 0u64;
    while text.len() < 10 * 1024 * 1024 {
        n += 1;
        let line = match n % 5 {
            0 => format!("    let việt_{n} = tính({n}, \"chuỗi {}\");   ", n * 7),
            1 => format!("// ghi chú {n} — e\u{301} 👩‍💻"),
            2 => String::new(),
            3 => format!("fn hàm_{}(x: i32) -> i32 {{ x + {n} }}", n % 997),
            _ => format!("\tsum += {n}; // total"),
        };
        text.push_str(&line);
        text.push_str("\r\n");
    }
    text
}

/// Median/max of three runs of `op` on a freshly prepared document; the
/// document construction is not timed.
fn time(
    text: &str,
    label: &str,
    prepare: impl Fn(&mut Document),
    op: impl Fn(&mut Document) -> String,
) {
    let mut samples = Vec::new();
    let mut note = String::new();
    for _ in 0..3 {
        let mut document = Document::from_text(text);
        document.prepare_view(4);
        prepare(&mut document);
        let start = Instant::now();
        note = op(&mut document);
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    samples.sort_by(f64::total_cmp);
    println!(
        "{label:<36} median {:>8.2} ms  max {:>8.2} ms  {note}",
        samples[1], samples[2]
    );
}

fn main() {
    let text = corpus();
    let lines = Document::from_text(&text).buffer().len_lines();
    println!("corpus: {} bytes, {lines} lines", text.len());
    let none = |_: &mut Document| {};
    let all = |d: &mut Document| d.select_all();
    let ops = [
        ("sort-lines (whole doc)", LineOperation::SortAscending),
        ("remove-duplicate-lines", LineOperation::RemoveDuplicates),
        ("remove-empty-lines", LineOperation::RemoveEmpty),
        ("trim-trailing", LineOperation::TrimTrailing),
    ];
    for (label, op) in ops {
        time(&text, label, none, |d| {
            d.apply_line_operation(op).unwrap();
            String::new()
        });
    }
    time(
        &text,
        "undo after sort-lines",
        |d| {
            d.apply_line_operation(LineOperation::SortAscending)
                .unwrap();
        },
        |d| {
            d.undo().unwrap();
            String::new()
        },
    );
    time(&text, "upper-case (select all)", all, |d| {
        d.convert_case(CaseConversion::Upper).unwrap();
        String::new()
    });
    time(&text, "toggle-comment (select all)", all, |d| {
        d.toggle_line_comment("//").unwrap();
        String::new()
    });
    time(&text, "indent (select all)", all, |d| {
        d.indent_lines("    ", 4, false).unwrap();
        String::new()
    });
    time(&text, "select-all + type one char", all, |d| {
        d.insert("x").unwrap();
        String::new()
    });
    time(&text, "selected_text (copy all)", all, |d| {
        format!("{} bytes", d.selected_text().len())
    });
    time(&text, "paste 1 MiB at start", none, |d| {
        d.insert(&text[..1024 * 1024]).unwrap();
        String::new()
    });
    time(
        &text,
        "duplicate line (caret at end)",
        |d| {
            let end = d.buffer().len_chars();
            d.select(end, end).unwrap();
        },
        |d| {
            d.duplicate().unwrap();
            String::new()
        },
    );
    time(
        &text,
        "type one char at end",
        |d| {
            let end = d.buffer().len_chars();
            d.select(end, end).unwrap();
        },
        |d| {
            d.insert("x").unwrap();
            String::new()
        },
    );
    time(&text, "find_with absent (worker)", none, |d| {
        format!("{}", d.find_with("absent-zzz", SearchOptions::default()))
    });
    time(&text, "replace_all 'total' (worker)", none, |d| {
        let n = d
            .replace_all_with("total", "sum", SearchOptions::default())
            .unwrap();
        format!("{n} replaced")
    });
    // Up/Down into and out of an unindexed 10 MiB Unicode line.
    let unit = "a e\u{301} 界 👩‍💻 ";
    let long = format!(
        "short line\n{}\nshort tail",
        unit.repeat(10 * 1024 * 1024 / unit.len())
    );
    time(
        &long,
        "Down into a long line (col 3)",
        |d| d.select(3, 3).unwrap(),
        |d| {
            d.move_cursor(Movement::Down, false);
            String::new()
        },
    );
    time(
        &long,
        "Up into a long line from its end",
        |d| {
            d.move_cursor(Movement::Down, false);
            d.move_cursor(Movement::End, false);
            d.move_cursor(Movement::Down, false);
        },
        |d| {
            d.move_cursor(Movement::Up, false);
            String::new()
        },
    );
    time(
        &long,
        "Down from a long line's end (unindexed)",
        |d| {
            d.move_cursor(Movement::Down, false);
            d.move_cursor(Movement::End, false);
        },
        |d| {
            d.move_cursor(Movement::Down, false);
            "(TUI queues this until the index is built)".into()
        },
    );
}
