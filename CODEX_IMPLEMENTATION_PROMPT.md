# Prompt triển khai GuineaNote dành cho Codex

Bạn là kỹ sư Rust cấp cao chịu trách nhiệm kiến trúc, phát triển và kiểm thử **GuineaNote**, một trình soạn thảo văn bản TUI chạy trực tiếp trong terminal, thân thiện như Notepad++/Nano. Hãy làm việc trong repository hiện tại.

## Nguồn yêu cầu bắt buộc

Đọc toàn bộ `GUINEANOTE_TECHNICAL_SPEC.md` (hoặc `docs/GUINEANOTE_TECHNICAL_SPEC.md` nếu tài liệu được đặt trong docs) trước khi viết mã. Tài liệu đó là nguồn yêu cầu chuẩn. Nếu repository đã có mã, trước hết khảo sát hiện trạng và bảo toàn phần đang hoạt động; không ghi đè bừa bãi.

## Quyết định đã chốt

- Tên: GuineaNote; binary chính `guineanote`.
- Rust stable, Cargo Workspace, MIT.
- TUI: Ratatui + Crossterm.
- Text engine MVP: Ropey sau `TextBuffer` abstraction có kiểm thử; cho phép thay backend trong tương lai.
- Phạm vi MVP: text editor trước; **không triển khai LSP, plugin runtime, Git integration hoặc large-file paged engine trong MVP**.
- Nền tảng: Windows, Linux, macOS; chạy SSH và terminal không có chuột.
- Không yêu cầu Node.js khi chạy binary. npm là kênh phân phối sau MVP.
- Độ đúng dữ liệu, Unicode, encoding, line endings và an toàn ghi file quan trọng hơn tốc độ hoặc số lượng tính năng.
- Không sao chép mã nguồn GPL của Fresh Editor vào dự án MIT. Chỉ tham khảo ý tưởng công khai.

## Quy trình thực hiện

1. **Khảo sát repository:** liệt kê cấu trúc, trạng thái git, dependencies, CI, test; xác định code đã có và phần còn thiếu. Không xóa thay đổi của người dùng.
2. **Lập kế hoạch M0–M4:** tạo `docs/IMPLEMENTATION_PLAN.md` gồm thứ tự công việc, dependencies, rủi ro, test và DoD; đối chiếu đặc tả. Nếu có quyết định mơ hồ, chọn giải pháp an toàn tối thiểu và ghi lại; chỉ hỏi khi thực sự bị chặn.
3. **Triển khai tuần tự:** M0 Foundation → M1 Text Core → M2 File Safety → M3 Usable TUI → M4 Hardening. Không nhảy sang M5. Mỗi bước giữ project compile và test được.
4. **Thiết kế API trước UI:** text editing chỉ thông qua core command/buffer; không gọi `ropey::Rope` trực tiếp từ UI. Tách byte/char/grapheme/cell/UTF-16 coordinate.
5. **Kiểm thử liên tục:** viết unit/property/golden/integration test cho các invariants; kiểm tra thao tác với tiếng Việt, emoji, mixed endings, UTF-8 BOM, invalid UTF-8 và ghi file thất bại.
6. **Kiểm tra trên hệ điều hành hiện có:** chạy format, clippy warnings-as-errors, tests, release build và smoke test. Những nền tảng chưa truy cập được phải ghi là `NOT TESTED`, không đoán PASS.
7. **Báo cáo sau mỗi milestone:** cập nhật `docs/IMPLEMENTATION_STATUS.md` với kết quả thực tế, vấn đề, giới hạn và bước kế tiếp.

## Yêu cầu kỹ thuật cụ thể

- `TerminalGuard` khôi phục raw mode, alternate screen, mouse state khi thoát/panic trong phạm vi khả thi.
- `Document` quản lý buffer, path, encoding, BOM, line endings, readonly, dirty/savepoint, selection và undo history.
- Cursor không được nằm giữa grapheme cluster; render dùng terminal cell width.
- UTF-8 là encoding chỉnh sửa được ở MVP; không tự động decode lossy hoặc ghi đè encoding không hỗ trợ.
- Mở/lưu không sửa phải roundtrip byte-for-byte với fixtures UTF-8 BOM, LF, CRLF, mixed line endings.
- Save-as/overwrite phải xử lý file bị sửa ngoài, readonly, permission denied, disk full và lỗi replace; khi thất bại phải bảo toàn file gốc và dirty state.
- Các phím tắt không truyền được qua một số terminal phải có fallback.
- Không busy-spin khi idle; không block event loop bằng tác vụ dài.
- Không dùng `unsafe` nếu chưa có lý do được ghi rõ và test riêng.
- Hạn chế dependencies, tránh framework/plugin thừa, không đưa LSP vào MVP.

## Tiêu chí nghiệm thu

Đạt toàn bộ AC-01 đến AC-15 trong đặc tả, hoặc ghi rõ BLOCKED/NOT TESTED và lý do. Không được tự tuyên bố hoàn thành nếu chưa có bằng chứng. Benchmark với tập dữ liệu và môi trường ghi rõ; không tự tuyên bố hỗ trợ file 1GB hoặc RAM bounded khi vẫn dùng Ropey resident-memory.

## Lệnh kiểm tra bắt buộc

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release
```

Thực hiện thêm integration, PTY, fault-injection và benchmark khi môi trường cho phép; nếu thiếu công cụ thì ghi rõ cách tái hiện.

## Deliverables

- Source Rust chạy được và có Cargo.lock.
- `LICENSE` MIT, `README.md` có quick start, keybindings, limitations.
- `docs/ARCHITECTURE.md`, `docs/EDITOR_BEHAVIOR.md`, `docs/FILE_SAFETY.md`, `docs/TEST_PLAN.md`, `docs/PERFORMANCE.md`.
- `docs/IMPLEMENTATION_PLAN.md`, `docs/IMPLEMENTATION_STATUS.md`.
- Unit, property, golden, integration tests và CI matrix.
- Binary release build trên môi trường hiện có; hướng dẫn cross-platform build.
- Báo cáo cuối: đã làm/chưa làm, kết quả test thực tế, hạn chế, rủi ro, đề xuất milestone tiếp theo.

## Bắt đầu ngay

Bắt đầu bằng khảo sát repository, lập kế hoạch M0–M4, sau đó **triển khai M0 và M1**. Nếu đủ thời gian và các quality gates đạt, tiếp tục M2–M4 theo thứ tự. Không dừng ở việc viết kế hoạch nếu môi trường cho phép sửa mã. Không thay đổi phạm vi đã chốt mà không ghi lại lý do. Ưu tiên các commit nhỏ, có thể review, không tự ý publish npm hoặc tạo release công khai.
