# Tên sản phẩm và migration executable

Quyết định Notion ngày 10/11/2026: **Guinea Note**, lệnh **`guinea`**.
[Nguồn](https://app.notion.com/p/41127da1dc088160b076e368c4858aad).

```sh
cargo build --workspace --release --locked
cargo run --bin guinea -- --language vi .
guinea main.rs
guinea .
guinea --help
guinea --version
```

Chạy trực tiếp `target/release/guinea` hoặc `target\release\guinea.exe` khi chưa
cài vào PATH. Cargo package vẫn `guineanote`; root manifest khai báo duy nhất
binary `guinea`, `autobins=false`. Các thư viện giữ nguyên ID/version.
Không có alias `guinote`/`guineanote` trong release mới. Người dùng script cũ
cần đổi riêng tên executable editor; binary cũ còn trong PATH/build cache có
thể vẫn chạy phiên bản cũ. Không tự xóa artifact hoặc sửa PATH của người dùng.

## Compatibility được giữ

- `--config guineanote.example.toml`, schema TOML và ưu tiên language/preferences.
- `%APPDATA%/GuineaNote/preferences.toml`, XDG guineanote/preferences.toml,
  recovery store và các biến GUINEANOTE_* hiện có.
- Binary `guineanote-agent`, `guineanote-mcp`, IPC session registry/endpoint,
  capability/secret policy và URI `guineanote://` giữ nguyên để không phá client.
- Nội dung file, BOM/EOL, draft provenance, save/history không migration bytes.
- Repository URL, Cargo/npm package ID và research/raw evidence lịch sử giữ nguyên.

Không có packaging/npm hoặc release scripts đang hoạt động để đổi mapping.
Khi bổ sung wrapper, bin mapping phải là `guinea`; cần kiểm tra tên package
và npm exec thực tế trước phát hành. Không suy ra registry availability.

## Gate nghiệm thu

CLI tests dùng `CARGO_BIN_EXE_guinea`, kiểm tra help/version, invalid options,
config validation và từ chối môi trường không có terminal trước khi phát ANSI.
Native PTY chạy binary mới cho open/save byte-exact, Settings/preferences, project,
Agent/MCP và recovery. Native Windows được ghi trong evidence của checkpoint này.
Linux/macOS mới có cross-Clippy; chưa chứng nhận runtime/release/ARM.

Native release matrix phải sinh guinea/guinea.exe, kiểm tra SHA256, notices/SBOM,
install/upgrade/rollback và smoke qua executable đó. Đây là gate tương lai;
không tạo/bật GitHub Actions và không publish package trong đợt migration này.
