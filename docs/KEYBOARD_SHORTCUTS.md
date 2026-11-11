# Phím tắt trong giao diện Guinea Note

**F1** mở hướng dẫn Việt/Anh; dùng ↑↓, PageUp/PageDown, Home/End để cuộn cả khi
cửa sổ chỉ 30×9. **F2** mở danh sách thao tác: mũi tên chọn, Enter chạy, gõ để
lọc. Không cần nhập lệnh CLI để sử dụng các thao tác trong TUI.

## Thẻ và file

| Thao tác | Phím |
| --- | --- |
| Quản lý toàn bộ thẻ | **Ctrl+P / F12** |
| Thẻ trước / sau | **F6 / F7** hoặc **Alt+← / Alt+→** |
| Chuyển thẻ bổ sung | Ctrl+PageDown / Ctrl+PageUp; Ctrl+Tab / Ctrl+Shift+Tab nếu terminal truyền được |
| Tạo / đóng thẻ | Ctrl+T hoặc Ctrl+N / Ctrl+W |
| Mở file / chọn thư mục | Ctrl+O / Ctrl+K |
| Lưu / lưu thành | Ctrl+S / **Alt+S** hoặc Ctrl+Shift+S |
| Đến dòng[:cột grapheme], 1-based | **Ctrl+L** |
| Thoát | Ctrl+Q |

Trong quản lý thẻ: gõ tên/đường dẫn để lọc; Ctrl+U xóa lọc. ↑↓ hoặc Tab/Shift+Tab
chọn, Enter chuyển, Ctrl+S lưu, Ctrl+W đóng, Ctrl+N tạo. Esc/Ctrl+P/F12 đóng
danh sách. Thẻ ngoài dải đang hiển thị vẫn chọn được trong danh sách này.

## Một bảng Find/Replace

**Ctrl+F và Ctrl+H mở cùng một bảng**, luôn có ô tìm và ô thay. Trong bảng:

| Thao tác | Phím |
| --- | --- |
| Tới ô tìm / ô thay và chọn nội dung ô | Ctrl+F / Ctrl+H |
| Qua ô nhập, kết quả và các nút | **Tab / Shift+Tab** |
| Kích hoạt nút đang focus | **Enter / Space** |
| Kết quả sau / trước | F3 / Shift+F3; Enter / Shift+Enter ở ô nhập; ↓ / ↑ |
| Về vị trí kết quả trong editor | Enter ở danh sách hoặc nút Go |
| Thay một / tất cả | **Ctrl+R / Alt+A** |
| Thay tất cả, tổ hợp cũ | Ctrl+Shift+R nếu terminal phân biệt được |
| Chọn hết / xóa ô | Ctrl+A / Ctrl+U |
| Đóng, hủy công việc chưa áp dụng | Esc |

Thứ tự Tab: **Tìm → Thay bằng → Kết quả → Trước → Sau → Go → Thay → Tất cả**.
Nút focus được tô rõ và giữ trong vùng nhìn thấy ở cửa sổ hẹp. Bố cục nhỏ dùng
`F ›` / `R ›` cho hai ô. Ctrl+F rồi Tab cũng tới ô thay nếu Ctrl+H không được
terminal truyền đúng. Ô thay trống sẽ xóa phần khớp khi chủ động thay thế.
Esc về editor, Ctrl+Z hoàn tác, Ctrl+S lưu riêng. File chỉ đọc/thẻ AI vẫn được bảo vệ.

## Các khu vực khác

| Khu vực | Phím |
| --- | --- |
| Soạn thảo | Ctrl+Z/Y hoàn tác/làm lại; Ctrl+A chọn hết; Ctrl+C/X/V sao chép/cắt/dán nội bộ; Shift+mũi tên chọn |
| Cây thư mục | Ctrl+B ẩn/hiện; F4 đổi khung; ↑↓ chọn, → mở nhánh, ← thu, Enter mở; Tab/Esc về editor; F5 đọc lại |
| Project | Ctrl+E tìm file; Ctrl+G tìm nội dung đĩa; Tab đổi chế độ; ↑↓/Page chọn, Enter mở, Ctrl+U xóa, Esc đóng |
| Chọn file | ↑↓/Page chọn; Enter mở; Tab điền; Alt+↑ thư mục cha; Ctrl+U xóa; Esc hủy |
| Xem trước | F8 chọn; ↑↓ hoặc Tab/Shift+Tab chọn, Enter áp dụng; mũi tên/Page/Home/End cuộn; Esc về nguồn |
| Mục lục Markdown | Alt+M ẩn/hiện; Tab đổi focus, ↑↓ chọn, Enter tới đề mục |
| Thiết lập | F10; mũi tên/Tab/Shift+Tab chọn, Enter áp dụng, Esc hủy |
| Thông tin file | F9; mũi tên/Page/Home/End cuộn, Enter/Esc đóng |
| Ô nhập đường dẫn/đến dòng | Ctrl+U xóa, Enter thực hiện, Esc hủy |
| Chế độ tập trung | F11 |

Xác nhận mặc định **Tiếp tục/Keep working**. Tab/←→ chọn quyết định; Enter chạy,
Y xác nhận bỏ, N/Esc hủy. Phím chuyển thẻ không đổi lựa chọn trong hộp xác nhận.
Esc đóng hộp nhập file/lưu/xác nhận trước khi chuyển thẻ. Với search/project,
phím chuyển thẻ đóng bảng an toàn; replacement đang áp dụng giữ khóa tới khi
hoàn tất hoặc Esc hủy.

Kiểm chứng Windows/ConPTY với `mouse = false`: `python -X utf8 scripts/pty_keyboard.py`.
F6/F7 và Alt+←/→ dùng được khi terminal giữ Ctrl+Tab. Runtime Linux/macOS/SSH
vẫn được ghi riêng theo phạm vi dự án.
