# GuineaNote — Đặc tả kỹ thuật

**Phiên bản tài liệu:** 1.0  
**Ngày:** 2026-11-07  
**Trạng thái:** Đặc tả triển khai MVP  
**License dự án:** MIT  
**Tên ứng dụng:** GuineaNote (`guineanote`, bí danh CLI dự kiến `gn`, chỉ bật nếu không xung đột)  
**Stack:** Rust stable, Cargo Workspace, Ratatui, Crossterm, Ropey

## 1. Tầm nhìn và giới hạn

GuineaNote là trình soạn thảo văn bản và mã nguồn chạy trong terminal (TUI), có thao tác thân thiện với người dùng Notepad++/Nano: phím tắt quen thuộc, menu, tab, tìm kiếm, tô màu cú pháp, trạng thái rõ ràng. Chạy trên Windows Terminal/PowerShell, Linux terminal/SSH và macOS terminal. Không yêu cầu Node.js lúc chạy binary.

**Nguyên tắc ưu tiên:** (1) không mất dữ liệu, (2) tính đúng đắn Unicode/encoding/line endings, (3) phản hồi tương tác, (4) trải nghiệm đơn giản, (5) mở rộng được. Không đánh đổi tính toàn vẹn file để tăng tốc. Không tuyên bố hỗ trợ file lớn trước khi đo đạc.

**Trong MVP:** mở/tạo/lưu file UTF-8, hỗ trợ BOM UTF-8, LF/CRLF, nhiều tab, chỉnh sửa, chọn vùng, clipboard nội bộ, undo/redo, tìm kiếm/thay thế literal, thanh trạng thái, syntax highlighting cơ bản, cấu hình, CLI, lỗi rõ ràng.

**Ngoài MVP:** LSP, Git integration, plugin runtime, multi-cursor, split panes, regex replace nâng cao, hỗ trợ file 1GB với bounded memory, remote filesystem riêng, collaborative editing. Có thể làm sau theo milestone.

## 2. Hợp đồng CLI và UX

- `guineanote [FILE ...]`: mở một hoặc nhiều file; đường dẫn không tồn tại tạo buffer mới chưa lưu, chỉ tạo file khi người dùng lưu.
- `guineanote --help`, `--version`, `--readonly`, `--config PATH`.
- `guineanote .`: chế độ duyệt thư mục **sau MVP**; MVP trả thông báo có hướng dẫn nếu chưa hỗ trợ.
- `gn`: alias tùy chọn khi kiểm tra không trùng lệnh hệ thống; tên chính thức luôn là `guineanote`.
- Phím tắt mặc định: Ctrl+S lưu, Ctrl+O mở, Ctrl+N mới, Ctrl+W đóng tab, Ctrl+Q thoát, Ctrl+F tìm, Ctrl+H thay thế, Ctrl+Z hoàn tác, Ctrl+Y làm lại, Ctrl+A chọn hết, Ctrl+C/X/V copy/cut/paste, Ctrl+P command palette (giai đoạn sau), Ctrl+Tab chuyển tab nếu terminal truyền được sự kiện, F1 trợ giúp. Luôn có fallback tương đương trong menu/command prompt cho các tổ hợp phím không được terminal hỗ trợ.
- Trạng thái hiển thị: tên file, dấu thay đổi `*`, readonly, encoding, BOM, line ending, ngôn ngữ, dòng/cột theo quy ước 1-based, trạng thái thông báo.
- Mọi hành động phá hủy dữ liệu (đóng buffer bẩn, thoát, reload, ghi đè thay đổi bên ngoài) cần xác nhận rõ ràng. Không tự ý ghi đè.
- Hỗ trợ resize terminal, terminal không có mouse, và fallback không có true color.

## 3. Kiến trúc hệ thống

```text
Terminal / SSH / Windows Terminal
            |
      Crossterm adapter
            |
     Event loop / commands  <--- background event channel
            |
   App state + Document manager
            |
       TextBuffer trait
            |
    RopeyBuffer (MVP)        Future: Paged/PieceTableBuffer
            |
    Text coordinates / undo / selection
            |
  Ratatui renderer + viewport diff
            |
  File service / encoding / atomic write
            |
  Syntax service (basic MVP; Tree-sitter later)
```

**Quy tắc phân lớp:** UI không được thao tác trực tiếp với `ropey::Rope`; mọi chỉnh sửa đi qua command + buffer API. Các thay đổi phải cập nhật trạng thái dirty, lịch sử undo, selection và viewport theo một giao dịch logic. Main event loop là chủ sở hữu duy nhất của trạng thái editor; background tasks gửi message, không mutate document trực tiếp. Không để I/O hoặc parse kéo dài chặn render/event loop.

### 3.1 Cargo Workspace

```text
guineanote/
├── Cargo.toml
├── Cargo.lock
├── LICENSE
├── README.md
├── docs/
│   ├── ARCHITECTURE.md
│   ├── EDITOR_BEHAVIOR.md
│   ├── FILE_SAFETY.md
│   ├── TEST_PLAN.md
│   └── PERFORMANCE.md
├── crates/
│   ├── guinea-core/        # Buffer trait, Ropey impl, document, selection, undo
│   ├── guinea-io/          # File read/write, encoding, BOM, line endings
│   ├── guinea-tui/         # Terminal events, rendering, tabs, dialogs
│   └── guinea-syntax/      # Language detection, highlighting interfaces
├── src/main.rs            # CLI binary
├── tests/                 # End-to-end / integration / golden fixtures
├── benches/               # Benchmarks
├── packaging/npm/         # npm wrapper; future release
└── .github/workflows/     # CI
```

### 3.2 Buffer abstraction

Không ép abstraction quá tổng quát ngay từ đầu. Thiết kế trait đủ cho các tác vụ thực sự dùng trong MVP:

```rust
pub trait TextBuffer {
    fn len_chars(&self) -> usize;
    fn len_lines(&self) -> usize;
    fn line(&self, line_idx: usize) -> Option<String>;
    fn insert(&mut self, char_idx: usize, text: &str) -> Result<(), EditError>;
    fn delete(&mut self, char_range: std::ops::Range<usize>) -> Result<String, EditError>;
    fn slice(&self, char_range: std::ops::Range<usize>) -> Result<String, EditError>;
}
```

Đây là API minh họa, Codex được phép tối ưu kiểu trả về để tránh cấp phát khi render. Cần ghi rõ độ phức tạp, quyền sở hữu và giới hạn. `RopeyBuffer` là implementation mặc định. Chỉ đưa vào paged buffer sau khi có test chung cho các implementation.

### 3.3 Tọa độ và Unicode

Phân biệt tuyệt đối: byte offset (file/UTF-8), Unicode scalar index (Ropey), grapheme cluster (cursor thao tác), terminal cell width (render), UTF-16 code units (LSP tương lai). Không dùng một chỉ số `column` cho mọi hệ tọa độ. Cung cấp module `coordinates` với chuyển đổi có kiểm tra biên. Cursor di chuyển theo grapheme cluster, không cắt giữa ký tự kết hợp/emoji ZWJ. Render xử lý full-width, tab, control characters, combining marks và clipping theo cell. Tính đúng đắn trên Windows và Linux phải có fixture.

### 3.4 Document và lịch sử chỉnh sửa

`Document` chứa: id, path tùy chọn, buffer, dirty flag, readonly, encoding, BOM, line ending policy, saved fingerprint, cursor/selection, undo/redo stacks. Các thao tác liên tiếp có thể gom undo transaction, nhưng phải định nghĩa ranh giới nhóm (typing burst, paste, replace-all). Undo/redo khôi phục đúng văn bản và selection; lưu file không xóa lịch sử. Dirty dựa trên savepoint/history hoặc checksum tin cậy, không chỉ bật/tắt thủ công.

### 3.5 I/O và an toàn dữ liệu

- MVP hỗ trợ UTF-8 không BOM/có BOM. UTF-16/ANSI/encoding khác: phát hiện khi có thể và **từ chối mở để chỉnh sửa** hoặc mở read-only có thông báo; không được tự động giải mã lossy rồi lưu đè. Mở rộng encoding sau khi có policy rõ ràng.
- Nhận diện LF, CRLF và mixed endings; bảo toàn chính xác nội dung ban đầu khi mở/lưu không sửa. Không tự động chuẩn hóa line endings; khi sửa một dòng trong file mixed-ending cần quy tắc chèn newline được tài liệu hóa và test.
- Phát hiện NUL/binary nghi vấn; không cho lưu đè qua text editor nếu chưa xác nhận chính sách.
- Lưu an toàn: tạo temp file cùng thư mục, ghi đầy đủ, flush/sync theo khả năng nền tảng, thay thế file đích bằng thao tác an toàn phù hợp OS; xử lý permission, metadata và symlink thận trọng. Không được khẳng định `rename` là atomic trên mọi filesystem/Windows; nếu không thể bảo đảm thì cảnh báo và giữ file gốc. Không âm thầm xóa file gốc khi thất bại.
- Trước khi ghi đè, so sánh fingerprint (mtime/size, bổ sung hash khi cần) để phát hiện file đã thay đổi bên ngoài; cho phép reload, save-as, cancel. Nếu file đã thay đổi, không tự ghi đè.
- File read-only và permission denied phải hiện lỗi có thể hành động; thao tác lưu thất bại không xóa dirty flag.
- Không đọc toàn bộ file nhiều lần vô ích; song MVP Ropey có thể cần resident memory. Đặt cảnh báo file lớn theo cấu hình, cho phép hủy mở.
- Crash recovery/autosave là tính năng sau MVP; không hứa bảo đảm khôi phục khi chưa triển khai.

### 3.6 TUI và event loop

`TerminalGuard` dùng RAII để bật/tắt raw mode, alternate screen, mouse capture và khôi phục terminal khi thoát thông thường/panic trong giới hạn có thể. Không swallow panic. Không vẽ lại toàn màn hình khi không thay đổi; debounce resize, throttle background events; không để vòng lặp busy-spin. Chuột chỉ là phương thức bổ sung; mọi thao tác chính có đường dùng bàn phím.

### 3.7 Syntax và extension

MVP: xác định ngôn ngữ theo extension và syntax highlighting cơ bản với bộ grammar/rule có kiểm thử. Tree-sitter incremental là giai đoạn sau MVP nếu tích hợp khiến MVP phình phạm vi. `SyntaxProvider` nhận snapshot hoặc revision của buffer, trả highlight spans có revision; kết quả quá hạn phải bỏ, không áp lên nội dung mới. LSP chỉ thiết kế interface, không khởi tạo runtime trong MVP.

## 4. Dependency và bảo mật

- `ratatui`, `crossterm`, `ropey`, `clap`, `serde`, `toml`, `thiserror`, `anyhow`, `unicode-segmentation`, `unicode-width`; cân nhắc `regex`, `tracing`, `tempfile`, `proptest`, `criterion` theo nhu cầu.
- Pin/lock dependencies bằng Cargo.lock cho ứng dụng; kiểm tra license và security advisories trong CI.
- Không sử dụng `unsafe` trong code dự án nếu không có RFC riêng và test bắt buộc.
- Không chạy lệnh shell dựa trên nội dung file; không tự tải plugin hay thực thi code từ project.
- MIT áp dụng cho mã GuineaNote tự viết; các dependencies có giấy phép riêng. Không sao chép mã Fresh Editor/GPL vào dự án MIT.

## 5. Milestones và Definition of Done

### M0 — Foundation (2–4 ngày)
- Cargo Workspace, CLI `--help/--version`, app boot/shutdown, terminal guard, CI Windows/Linux/macOS, lint/test/format, README/LICENSE.
- **DoD:** `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace` đều pass; chạy mở/thoát khôi phục terminal.

### M1 — Text Core (5–8 ngày)
- RopeyBuffer, grapheme cursor, selection, insert/delete/paste, undo/redo, dirty/savepoint; unit/property tests.
- **DoD:** thao tác Unicode không panic, undo/redo roundtrip, không tách grapheme; buffer API độc lập UI.

### M2 — File Safety (4–7 ngày)
- Open/save/save-as, UTF-8 BOM, line endings, external change detection, readonly, failure handling.
- **DoD:** byte-for-byte roundtrip không sửa cho fixtures UTF-8 BOM/LF/CRLF/mixed; file gốc nguyên vẹn khi lỗi ghi; không tự chuyển encoding.

### M3 — Usable TUI MVP (6–10 ngày)
- Tabs, menu/help, status bar, navigation, search/replace literal, keyboard fallback, viewport scrolling, mouse optional, config.
- **DoD:** có thể soạn/sửa/lưu nhiều file trên Windows/Linux; cảnh báo unsaved; resize và clipboard nội bộ hoạt động; terminal khôi phục khi thoát.

### M4 — MVP Hardening / v0.1.0 (4–7 ngày)
- Syntax highlighting cơ bản, golden tests, PTY smoke tests, benchmarks, packaging binary, docs.
- **DoD:** tất cả test gates pass, không còn lỗi mất dữ liệu nghiêm trọng, release artifacts kiểm tra được, tài liệu giới hạn rõ ràng.

### M5 — Post-MVP (tách các PR)
- Tree-sitter incremental, regex replace, file explorer, multi-cursor, split panes, crash recovery, bounded-memory large-file backend, LSP, npm packaging.
- Không gom M5 vào M0–M4; ưu tiên theo benchmark và phản hồi sử dụng.

**Ước lượng tổng M0–M4:** 21–36 ngày công kỹ thuật, chưa tính thời gian chờ review, phát hiện bug hoặc test đa nền tảng; không phải cam kết lịch.

## 6. Acceptance criteria tổng hợp

| ID | Tiêu chí | Kiểm chứng |
|---|---|---|
| AC-01 | Binary mở và thoát sạch trên Windows/Linux/macOS | CI + smoke manual |
| AC-02 | Gõ, xóa, chọn, copy/paste, undo/redo đúng | Unit/property + PTY |
| AC-03 | Mở/lưu nhiều tab không lẫn nội dung | Integration |
| AC-04 | Không mất byte khi mở/lưu UTF-8 không sửa | Golden fixtures + byte compare |
| AC-05 | BOM, LF/CRLF/mixed endings được bảo toàn | Golden fixtures |
| AC-06 | Không cắt grapheme Unicode | Property tests |
| AC-07 | Ghi file thất bại không làm mất file gốc | Fault injection |
| AC-08 | Thay đổi bên ngoài không bị ghi đè âm thầm | Integration |
| AC-09 | Đóng/thoát buffer bẩn có xác nhận | PTY/UI test |
| AC-10 | Resize và terminal cleanup hoạt động | PTY + manual |
| AC-11 | Không busy-spin khi idle | CPU sampling/benchmark |
| AC-12 | Không phụ thuộc Node runtime | Binary smoke |
| AC-13 | Không chứa mã GPL sao chép từ Fresh | License/source review |
| AC-14 | `fmt`, `clippy -D warnings`, tests pass | CI |
| AC-15 | Giới hạn file lớn được hiển thị rõ | Manual + docs |

## 7. Chiến lược kiểm thử

**Unit:** text buffer boundary, newline, selection, cursor, coordinates, history, file encoding, search.

**Property-based:** chuỗi insert/delete/undo/redo ngẫu nhiên; invariants về Ropey char count, grapheme boundary; mở-lưu roundtrip cho dữ liệu hợp lệ.

**Golden fixtures:** ASCII, tiếng Việt NFC/NFD, emoji ZWJ, combining marks, CJK, tabs, empty file, no trailing newline, BOM, LF, CRLF, mixed endings, very long line, NUL/binary, invalid UTF-8.

**Integration:** tạo file tạm, read-only, file bị sửa bên ngoài, permission errors, path Unicode, path có khoảng trắng, đường dẫn tương đối/tuyệt đối, symlink, save-as, multi-tab.

**Fault injection:** disk full/short write, rename failure, permission denied, process interruption ở các giai đoạn ghi. Trường hợp không mô phỏng được trên CI phải ghi rõ test manual.

**PTY/UI:** keyboard shortcuts, dialog unsaved, search, tabs, resize, mouse optional, terminal state restoration.

**Benchmarks:** cold open, save, random edits, scrolling, search trên 1KB/1MB/10MB/100MB với môi trường đo ghi rõ; đo latency p50/p95, CPU, peak RSS, throughput. 100MB là workload đánh giá, không tự động trở thành mức hỗ trợ được bảo đảm. Mục tiêu p95 thao tác thông thường <20ms trên máy benchmark tham chiếu, cần đo và ghi điều kiện.

**CI matrix:** stable Rust Windows x64, Ubuntu x64, macOS (runner phù hợp); format, clippy, tests, release build, cargo audit/license checks nếu khả dụng. Không bỏ test chỉ để CI xanh.

## 8. Phát hành và npm

Binary native là nguồn chân lý. `cargo install guineanote` và `npm install -g guineanote` chỉ được công bố khi tên package được xác minh còn trống và pipeline đã có artifact thực tế. npm wrapper có thể dùng `optionalDependencies` theo nền tảng hoặc postinstall tải binary từ release có checksum; không tự động thực thi remote code tùy ý. Ưu tiên binary per-platform có checksum và provenance. Không bắt người dùng có Rust/Node sau khi đã cài binary.

## 9. Rủi ro và quyết định kiến trúc

1. **Unicode:** Ropey dùng char index, terminal dùng cell width — phải có conversion layer và test.
2. **Encoding:** chỉ UTF-8 ở MVP; không silent lossy conversion.
3. **Line endings:** giữ nguyên mixed endings; không normalize khi không yêu cầu.
4. **Atomic write:** khác biệt Windows/POSIX; test theo OS, không cam kết atomicity tuyệt đối.
5. **Large files:** Ropey resident memory; không hứa bounded memory; hậu MVP nghiên cứu paged/piece-table backend.
6. **Terminal keys:** một số terminal không gửi Ctrl+Tab/Ctrl+Shift+...; có fallback.
7. **Fresh Editor:** chỉ tham khảo trải nghiệm/kiến trúc công khai, không sao chép mã GPL.
8. **Scope creep:** LSP, plugin, Git, multi-cursor, recovery là post-MVP.

## 10. Handoff và báo cáo

Sau mỗi milestone Codex cập nhật `docs/IMPLEMENTATION_STATUS.md` với: phạm vi đã làm, commit/files thay đổi, lệnh kiểm thử và kết quả thật, lỗi còn tồn tại, giới hạn, benchmark (nếu có), quyết định kỹ thuật và công việc tiếp theo. Không tuyên bố pass nếu chưa chạy. Không xóa test thất bại hoặc giảm độ nghiêm ngặt để đạt milestone.
