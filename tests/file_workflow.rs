use guinea_core::Movement;
use std::fs;

#[test]
fn two_documents_have_independent_history_and_file_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let a_path = dir.path().join("tiếng Việt.txt");
    let b_path = dir.path().join("emoji file.txt");
    fs::write(&a_path, b"\xef\xbb\xbfa\r\nb\n").unwrap();
    fs::write(&b_path, "e\u{301} 👩‍💻\n").unwrap();
    let mut a = guinea_io::open(&a_path, false, 1000).unwrap();
    let mut b = guinea_io::open(&b_path, false, 1000).unwrap();
    assert_ne!(a.id, b.id);
    a.insert("Việt ").unwrap();
    b.move_cursor(Movement::Finish, false);
    b.insert("界").unwrap();
    guinea_io::save(&mut b, &b_path).unwrap();
    assert!(a.is_dirty());
    assert!(!b.is_dirty());
    assert_eq!(fs::read(&a_path).unwrap(), b"\xef\xbb\xbfa\r\nb\n");
    guinea_io::save(&mut a, &a_path).unwrap();
    assert_eq!(
        fs::read(&a_path).unwrap(),
        "\u{feff}Việt a\r\nb\n".as_bytes()
    );
    // A second save exercises the fingerprint recorded at replacement time.
    a.insert("again ").unwrap();
    guinea_io::save(&mut a, &a_path).unwrap();
    b.undo().unwrap();
    assert_eq!(b.text(), "e\u{301} 👩‍💻\n");
    assert!(b.is_dirty());
    assert!(!a.is_dirty());
}
