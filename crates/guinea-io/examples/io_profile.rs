//! Research timings for open/save and Undo retention on a ~10 MiB corpus.
//!     cargo run --release -p guinea-io --example io_profile
use guinea_core::{Document, LineOperation, TextBuffer};
use std::time::Instant;

fn rss_kib() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("VmRSS:"))
                .and_then(|l| l.split_whitespace().nth(1)?.parse().ok())
        })
        .unwrap_or(0)
}

fn main() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corpus.rs");
    let mut text = String::new();
    let mut n = 0u64;
    while text.len() < 10 * 1024 * 1024 {
        n += 1;
        text.push_str(&format!("    let việt_{n} = tính({n}); // e\u{301} 👩‍💻\r\n"));
    }
    std::fs::write(&path, &text).unwrap();
    println!("corpus {} bytes, rss start {} KiB", text.len(), rss_kib());
    for run in 0..3 {
        let start = Instant::now();
        let document = guinea_io::open(&path, false, u64::MAX).unwrap();
        let open = start.elapsed();
        let start = Instant::now();
        document.prepare_view(4);
        let prepare = start.elapsed();
        let mut document = document;
        document.insert("x").unwrap();
        let start = Instant::now();
        guinea_io::save(&mut document, &path).unwrap();
        println!(
            "run {run}: open {:.1} ms, prepare_view {:.1} ms, save {:.1} ms",
            open.as_secs_f64() * 1e3,
            prepare.as_secs_f64() * 1e3,
            start.elapsed().as_secs_f64() * 1e3
        );
    }
    let mut document = Document::from_text(&text);
    println!("rss with one 10 MiB document {} KiB", rss_kib());
    for step in 1..=8 {
        document.select_all();
        document.indent_lines("    ", 4, step % 2 == 0).unwrap();
        let status = document.history_status();
        println!(
            "after whole-file edit {step}: undo {} steps, charged {:.1} MiB, trimmed {}, rss {} KiB",
            status.undo,
            status.charged_bytes as f64 / (1 << 20) as f64,
            status.trimmed,
            rss_kib()
        );
    }
    let mut document = Document::from_text(&text);
    for _ in 0..3 {
        document.select_all();
        document
            .apply_line_operation(LineOperation::TrimTrailing)
            .unwrap();
        document.select(0, 0).unwrap();
        document.insert("y").unwrap();
    }
    println!(
        "typing after whole-file edits keeps {} undo steps; len {}",
        document.history_status().undo,
        document.buffer().len_bytes()
    );
}
