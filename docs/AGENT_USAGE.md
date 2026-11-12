# Sử dụng CLI và MCP cho AI Agent

Chạy ở chế độ dev từ thư mục dự án:

```powershell
cargo build --workspace
cargo run -- --agent --agent-write duong-dan/file.txt
```

`--agent` bật đọc cả nội dung chưa lưu; thêm `--agent-write` cho phép Agent
plan/apply/save và quản lý thẻ. Bỏ hai cờ để chạy editor thông thường. Mặc định
Agent không đọc `.env`, private key hoặc credentials; chỉ thêm `--agent-secrets`
khi muốn cấp quyền đó. Thư mục thông tin xác thực phiên luôn bị chặn.

Trong terminal khác, lấy phiên và thẻ đang mở:

```powershell
target/debug/guineanote-agent.exe --root . sessions list
target/debug/guineanote-agent.exe --root . --session SESSION_ID tabs list
target/debug/guineanote-agent.exe --root . --session SESSION_ID documents read --document DOCUMENT_ID
```

Thay `SESSION_ID`, `DOCUMENT_ID` bằng giá trị JSON trả về. `--root` phải khớp
workspace đã mở trong editor; có thể truyền đường dẫn tuyệt đối. ID tài liệu và
ID thẻ khác nhau, chỉ có hiệu lực trong phiên đó. Không truyền `--session` thì
CLI chỉ đọc file trên đĩa, không thấy bản nháp đang mở.

```powershell
target/debug/guineanote-agent.exe --root . documents analyze --path data.json --limit 20
target/debug/guineanote-agent.exe --root . files list src --limit 20
target/debug/guineanote-agent.exe --root . capabilities
```

CLI luôn trả JSON; lỗi nghiệp vụ có `error.code` và exit code 1. `--help` hiển
thị cách dùng của từng nhóm lệnh. Trang nội dung tối đa 64 KiB; dùng `next_start`
và `expected_hash` cho trang tiếp theo. Tọa độ edit là Unicode scalar bắt đầu
từ 0, khoảng nửa mở, hai đầu phải là ranh giới grapheme; không phải byte/UTF-16.

Quy trình ghi là **đọc → plan → xem diff → apply → save**. Mỗi lệnh ghi cần
`--access write`, đồng thời editor phải có `--agent-write`. Có thể gửi request
JSON bằng `request --file request.json`, ví dụ nội dung file:

```json
{
  "request_id": "plan-note-001",
  "op": "plan_document_patch",
  "document": "SESSION_ID:dDOCUMENT_NUMBER",
  "expected_revision": 3,
  "edits": [{"start": 0, "end": 0, "text": "# Title\n"}]
}
```

```powershell
target/debug/guineanote-agent.exe --root . --session SESSION_ID --access write request --file request.json
target/debug/guineanote-agent.exe --root . --session SESSION_ID --access write --request-id apply-note-001 patch apply PLAN_ID
target/debug/guineanote-agent.exe --root . --session SESSION_ID --access write --request-id save-note-001 documents save --document DOCUMENT_ID --expected-revision NEW_REVISION
target/debug/guineanote-agent.exe --root . --session SESSION_ID transaction save-note-001
```

Plan tạo thẻ **AI diff** chỉ đọc và giữ nguyên nguồn. Apply tạo một bước undo;
save là lệnh riêng. Revision cũ hoặc file đổi ngoài editor trả lỗi xung đột.
Lặp lại cùng `request_id` và cùng tham số trả lại kết quả ghi trước đó. Sau
disconnect, tra `transaction` trước khi thử ghi lại; JSON/MCP dùng tên tham số
`transaction_id`. Không tái sử dụng khóa cho một thao tác khác. Ledger tồn tại
trong bộ nhớ của phiên, tối đa 2.048 lệnh ghi; khởi động lại làm mất ledger.

Khi Agent giữ một tài liệu, thẻ nguồn và review liên kết hiện **AI**, trạng thái
**AI · CHỈ ĐỌC / AI · READ ONLY**. Người dùng vẫn cuộn, chọn, sao chép và chuyển
thẻ; nhập/xóa/undo/save/đóng thẻ đó bị chặn. Các thẻ khác vẫn chỉnh sửa được.
Hoàn tất, lỗi, hủy hoặc mất kết nối đều giải phóng khóa sau khi tác vụ dọn dẹp
an toàn. Một thẻ vốn chỉ đọc vẫn chỉ đọc sau khi Agent kết thúc.

MCP chạy bằng binary riêng, qua stdio:

```powershell
target/debug/guineanote-mcp.exe --root . --session SESSION_ID --access write
```

Đây là server chờ message JSON-RPC từ host. Ví dụ cấu hình kiểu `mcpServers`
(điều chỉnh theo host của bạn; thay ID mỗi lần mở phiên editor mới):

```json
{
  "mcpServers": {
    "guineanote": {
      "command": "D:\\Project\\GuineaNote\\target\\debug\\guineanote-mcp.exe",
      "args": ["--root", "D:\\Project\\GuineaNote", "--session", "SESSION_ID", "--access", "write"]
    }
  }
}
```

MCP công bố tools có schema và resources đọc phân trang. Lệnh ghi bắt buộc có
`request_id`. Chưa đăng ký tự động vào ứng dụng AI nào; kiểm chứng hiện tại dùng
client probe cục bộ cho giao thức 2025-11-25 và 2026-07-28. Xem
[phạm vi triển khai và bằng chứng](AGENT_IMPLEMENTATION.md) trước khi tích hợp.

Các phân tích cấu trúc Agent hiện gồm Markdown outline, JSON/notebook, XML,
CSV/TSV. Mọi file UTF-8 khác đọc/tìm/xem nguồn được; bản review TUI giữ các
renderer YAML, JSON5, ENV, INI, Properties, config và ngôn ngữ lập trình hiện có.
Nội dung file không được chạy như lệnh hoặc chỉ thị cấp quyền.
