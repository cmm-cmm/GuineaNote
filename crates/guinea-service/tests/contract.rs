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
    for xml in ["<a>", "<a/><b/>", "outside<a/>", "<a></b>"] {
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
