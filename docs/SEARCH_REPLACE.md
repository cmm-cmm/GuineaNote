# Tìm kiếm và thay thế / Search and replace

Mở bằng Ctrl+F hoặc Ctrl+H: **cùng một bảng, luôn có cả hai ô tìm và thay thế**.
Thanh công cụ và danh sách F2 chỉ có một mục Find/Replace. F3 tiếp tục tìm từ
khóa gần nhất. Khi bảng đang mở, Ctrl+F tới ô tìm, Ctrl+H tới ô thay thế.
Giao diện hỗ trợ tiếng Việt và tiếng Anh, theo F10 Settings.

1. Nhập từ khóa. Chương trình tìm tự động, hiển thị tổng số kết quả, số dòng
   và ngữ cảnh. Đây là tìm chuỗi nguyên văn, có phân biệt hoa/thường.
2. Bấm kết quả để chọn; bấm đúp hoặc Go to match để trở về vị trí trong editor.
   Previous/Next hoặc Enter/Shift+Enter chuyển kết quả, quay vòng ở hai đầu.
3. Nhấn Tab tới ô thay thế và nhập nội dung mới. Tab/Shift+Tab chuyển giữa hai
   ô, danh sách kết quả và từng nút; Enter/Space kích hoạt nút đang focus.
   Ctrl+A chọn toàn bộ ô đang nhập, Ctrl+U xóa ô. Ô thay thế trống sẽ
   xóa nội dung tìm thấy.
4. Ctrl+R thay một, **Alt+A thay tất cả**; Ctrl+Shift+R còn được hỗ trợ khi
   terminal truyền được. Có thể Tab tới nút Replace/Replace all rồi Enter.
   Một lần thay tạo một
   bước Undo; Ctrl+Z trong editor hoàn tác. Ctrl+S lưu riêng sau khi kiểm tra.
5. Esc hoặc `x` đóng panel. Công việc chưa được áp dụng sẽ bị hủy; thay đổi đã
   áp dụng vẫn có thể Undo. Panel không tự ghi dữ liệu xuống đĩa.

Khi mở, từ khóa lấy từ vùng văn bản một dòng đang chọn (tối đa 256 Unicode
scalars), hoặc dùng lần tìm gần nhất. Hai ô có thể bấm chọn bằng chuột, chỉnh
sửa bằng phím và dán văn bản. Kết quả đánh dấu từ khóa trong ngữ cảnh và hiển
thị preview thay thế. File chỉ đọc, review tab và file đang do AI thao tác
được bảo vệ khỏi thay thế; các yêu cầu cũ không được áp dụng vào phiên bản mới.

## Implementation and limits

Search uses literal matching at whole Unicode grapheme boundaries, not regular
expressions. It is case-sensitive by default; **Alt+C** (or clicking the option)
toggles case-insensitive matching and **Alt+W** toggles whole-word matching.
Options stay for the session and also apply to F3 and Replace all.
Case-insensitive matching uses simple one-to-one lowercase folding, so a match
always has the query's scalar length: `İ` and `ß` match only themselves.
Whole word requires a non-alphanumeric, non-underscore scalar (or the text
edge) on both sides of the match. Inputs are single-line and capped at
4,096 UTF-8 bytes each; pasted CR/LF/NUL characters are removed. Replacement
output is capped at 128 MiB. The measured interaction workload is up to 10 MiB.

Scanning and replacement use immutable document snapshots on at most two
background search workers, with a 35 ms input debounce. Results retain 200
matches per page plus a total count; previous/next traverses further pages
instead of silently truncating matches. Replacements validate the original
document ID, revision and access permissions before committing one prepared
edit on the UI thread. Existing path, fingerprint, BOM/newline handling and
save history are preserved. No source file is rewritten until an explicit save.

Cancellation is cooperative. Closing the panel discards pending worker results;
the worker may finish its current preparation before exiting. Background scan,
replacement and save completion are separate from initial UI feedback. Every
interactive response >50 ms requires investigation. Keyboard-only coverage
with mouse capture disabled is in `scripts/pty_keyboard.py`; search/replace
regression coverage is in `scripts/pty_search.py` and the
recorded timings are documented in [PERFORMANCE.md](PERFORMANCE.md).
