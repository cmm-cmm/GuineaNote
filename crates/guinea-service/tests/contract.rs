use guinea_service::{policy::Policy, *};
use serde_json::json;
use std::sync::{Arc, atomic::AtomicBool};
fn call(service: &mut Service, value: serde_json::Value) -> Response {
    let operation: Operation = serde_json::from_value(value).unwrap();
    service.handle(Request::new(operation), Arc::new(AtomicBool::new(false)))
}
#[test]
fn paginated_unicode_read_search_analysis_and_conflicts() {
    let dir = tempfile::tempdir().unwrap();
    let text = "e\u{301} 👩‍💻\r\n# Việt\r\nfind find";
    std::fs::write(dir.path().join("note.md"), text).unwrap();
    let mut service = Service::new(Policy::new(dir.path(), false, false).unwrap(), None);
    let first = call(
        &mut service,
        json!({"op":"read_document","path":"note.md","limit":16}),
    )
    .result()
    .unwrap();
    let mut combined = first["text"].as_str().unwrap().to_owned();
    let mut next = first["next_start"].as_u64();
    while let Some(start) = next {
        let page=call(&mut service,json!({"op":"read_document","path":"note.md","start":start,"limit":16,"expected_hash":first["hash"]})).result().unwrap();
        combined.push_str(page["text"].as_str().unwrap());
        next = page["next_start"].as_u64();
    }
    assert_eq!(combined, text);
    assert_eq!(
        call(
            &mut service,
            json!({"op":"read_document","path":"note.md","start":1})
        )
        .error
        .unwrap()
        .code,
        "INVALID_ARGUMENT"
    );
    let matches = call(
        &mut service,
        json!({"op":"search_document","path":"note.md","needle":"find","limit":1}),
    )
    .result()
    .unwrap();
    assert!(matches["truncated"].as_bool().unwrap());
    let second=call(&mut service,json!({"op":"search_document","path":"note.md","needle":"find","start":matches["next_start"]})).result().unwrap();
    assert_eq!(second["matches"].as_array().unwrap().len(), 1);
    let analysis = call(
        &mut service,
        json!({"op":"analyze_document","path":"note.md"}),
    )
    .result()
    .unwrap();
    assert_eq!(analysis["entries"][0]["title"], "Việt");
    std::fs::write(dir.path().join("note.md"), "changed").unwrap();
    assert_eq!(
        call(
            &mut service,
            json!({"op":"read_document","path":"note.md","expected_hash":first["hash"]})
        )
        .error
        .unwrap()
        .code,
        "REVISION_CONFLICT"
    );
}
#[test]
fn scope_secrets_limits_and_typed_schema() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join(".env"), "TOKEN=private").unwrap();
    std::fs::write(dir.path().join("public.txt"), "public").unwrap();
    let mut service = Service::new(Policy::new(dir.path(), false, false).unwrap(), None);
    for path in ["../outside", ".env", "C:drive", "file:ads"] {
        assert_eq!(
            call(&mut service, json!({"op":"get_file_info","path":path}))
                .error
                .unwrap()
                .code,
            "OUT_OF_SCOPE"
        );
    }
    let listing = call(&mut service, json!({"op":"list_files"}))
        .result()
        .unwrap();
    assert_eq!(listing["total"], 1);
    assert_eq!(
        call(
            &mut service,
            json!({"op":"read_document","path":"public.txt","limit":65537})
        )
        .error
        .unwrap()
        .code,
        "LIMIT_EXCEEDED"
    );
    assert_eq!(
        call(&mut service, json!({"op":"new_document"}))
            .error
            .unwrap()
            .code,
        "READ_ONLY"
    );
    let request: Request =
        serde_json::from_value(json!({"request_id":"request","op":"get_capabilities"})).unwrap();
    assert_eq!(request.operation.name(), "get_capabilities");
    let receipt: Request = serde_json::from_value(
        json!({"request_id":"query","op":"get_transaction","transaction_id":"write"}),
    )
    .unwrap();
    assert!(
        matches!(receipt.operation,Operation::GetTransaction{transaction_id} if transaction_id=="write")
    );
    assert!(
        serde_json::from_value::<Request>(
            json!({"request_id":"request","op":"get_capabilities","shell":"bad"})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<Operation>(json!({"op":"get_capabilities","shell":"bad"}))
            .is_err()
    );
    let schemas = tool_schemas();
    assert!(
        schemas
            .iter()
            .any(|(name, _)| name == "plan_document_patch")
    );
    for (_, schema) in schemas {
        assert_eq!(schema["additionalProperties"], false);
    }
}
#[test]
fn malformed_data_and_output_are_bounded() {
    use guinea_syntax::Language;
    for xml in [
        "<a>",
        "<a/><b/>",
        "outside<a/>",
        "<a></b>",
        "<a x='1' x='2'/>",
        "<a x=unquoted/>",
    ] {
        assert!(
            analysis::analyze(xml, Language::Xml, 0, 200).is_err(),
            "{xml}"
        );
    }
    assert!(analysis::analyze("a,b\nx", Language::Csv, 0, 200).is_err());
    let value = format!("\"{}\"", "x".repeat(200_000));
    let result = analysis::analyze(&value, Language::Json, 0, 200).unwrap();
    assert!(serde_json::to_vec(&result).unwrap().len() < 4096);
    let markdown = "# One\n\n## Two\n";
    let page = analysis::analyze(markdown, Language::Markdown, 1, 1).unwrap();
    assert_eq!(page["entries"][0]["title"], "Two");
    assert_eq!(page["total"], 2);
}

#[test]
fn response_budget_and_invalid_write_ids_do_not_fill_ledger() {
    let dir = tempfile::tempdir().unwrap();
    // Escaped control scalars expand sixfold in JSON, exercising the serialized cap.
    let text = ("\u{1}".repeat(512) + "\n").repeat(200);
    std::fs::write(dir.path().join("wide.txt"), text).unwrap();
    let mut service = Service::new(Policy::new(dir.path(), false, false).unwrap(), None);
    assert_eq!(
        call(
            &mut service,
            json!({"op":"analyze_document","path":"wide.txt"})
        )
        .error
        .unwrap()
        .code,
        "LIMIT_EXCEEDED"
    );
    assert!(
        call(
            &mut service,
            json!({"op":"analyze_document","path":"wide.txt","limit":1})
        )
        .error
        .is_none()
    );
    let invalid = "x".repeat(1024);
    let response = service.handle(
        Request {
            request_id: invalid.clone(),
            operation: Operation::NewDocument {},
        },
        Arc::new(AtomicBool::new(false)),
    );
    assert_eq!(response.request_id, "invalid-request-id");
    assert_eq!(response.error.unwrap().code, "INVALID_ARGUMENT");
    assert_eq!(
        call(
            &mut service,
            json!({"op":"get_transaction","transaction_id":invalid})
        )
        .error
        .unwrap()
        .code,
        "NOT_FOUND"
    );
    #[cfg(windows)]
    for path in [".env.", ".env ", "NUL", "COM1.txt", "lpt²", "dir."] {
        assert_eq!(
            call(&mut service, json!({"op":"get_file_info","path":path}))
                .error
                .unwrap()
                .code,
            "OUT_OF_SCOPE"
        );
    }
}

#[test]
fn native_directory_links_cannot_escape_workspace() {
    let root = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret.txt"), "outside").unwrap();
    let link = root.path().join("linked");
    #[cfg(windows)]
    {
        let created = std::process::Command::new("cmd")
            .args(["/d", "/c", "mklink", "/J"])
            .arg(&link)
            .arg(outside.path())
            .output()
            .unwrap();
        assert!(
            created.status.success(),
            "{}",
            String::from_utf8_lossy(&created.stderr)
        );
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(outside.path(), &link).unwrap();
    let mut service = Service::new(Policy::new(root.path(), false, false).unwrap(), None);
    assert_eq!(
        call(
            &mut service,
            json!({"op":"read_document","path":"linked/secret.txt"})
        )
        .error
        .unwrap()
        .code,
        "OUT_OF_SCOPE"
    );
    assert_eq!(
        call(&mut service, json!({"op":"list_files"}))
            .result()
            .unwrap()["total"],
        0
    );
    // Remove only the link itself. Never recursively traverse its target.
    #[cfg(windows)]
    std::fs::remove_dir(&link).unwrap();
    #[cfg(unix)]
    std::fs::remove_file(&link).unwrap();
    assert_eq!(
        std::fs::read_to_string(outside.path().join("secret.txt")).unwrap(),
        "outside"
    );
}
