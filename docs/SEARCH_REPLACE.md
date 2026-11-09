# Tìm kiếm và thay thế / Search and replace

Mở bằng Ctrl+F (tìm kiếm), Ctrl+H (tìm/thay thế), nút Find/Replace trên thanh
công cụ hoặc F2 → `find` / `replace`. F3 tiếp tục tìm từ khóa gần nhất.
Giao diện hỗ trợ tiếng Việt và tiếng Anh, theo F8 Settings.

1. Nhập từ khóa. Chương trình tìm tự động, hiển thị tổng số kết quả, số dòng
   và ngữ cảnh. Đây là tìm chuỗi nguyên văn, có phân biệt hoa/thường.
2. Bấm kết quả để chọn; bấm đúp hoặc Go to match để trở về vị trí trong editor.
   Previous/Next hoặc Enter/Shift+Enter chuyển kết quả, quay vòng ở hai đầu.
3. Chuyển sang Replace, nhập nội dung thay thế. Tab chuyển giữa hai ô và danh
   sách; Ctrl+A chọn toàn bộ ô đang nhập, Ctrl+U xóa ô. Ô thay thế trống sẽ
   xóa nội dung tìm thấy.
4. Bấm Replace (Ctrl+R) hoặc Replace all (Ctrl+Shift+R). Một lần thay tạo một
   bước Undo; Ctrl+Z trong editor hoàn tác. Ctrl+S lưu riêng sau khi kiểm tra.
5. Esc hoặc `x` đóng panel. Công việc chưa được áp dụng sẽ bị hủy; thay đổi đã
   áp dụng vẫn có thể Undo. Panel không tự ghi dữ liệu xuống đĩa.

Khi mở, từ khóa lấy từ vùng văn bản một dòng đang chọn (tối đa 256 Unicode
scalars), hoặc dùng lần tìm gần nhất. Hai ô có thể bấm chọn bằng chuột, chỉnh
sửa bằng phím và dán văn bản. Kết quả đánh dấu từ khóa trong ngữ cảnh và hiển
thị preview thay thế. File chỉ đọc, review tab và file đang do AI thao tác
được bảo vệ khỏi thay thế; các yêu cầu cũ không được áp dụng vào phiên bản mới.

## Implementation and limits

Search uses literal, case-sensitive matching at whole Unicode grapheme
boundaries, not regular expressions. Inputs are single-line and capped at
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
replacement and save completion are separate from the <100 ms measured UI
feedback budget. Native ConPTY coverage is in `scripts/pty_search.py` and the
recorded timings are documented in [PERFORMANCE.md](PERFORMANCE.md).
