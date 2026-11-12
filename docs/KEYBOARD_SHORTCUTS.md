# Phím tắt trong giao diện Guinea Note — bảng phím v1

Bảng phím v1 được chốt ngày 12/11/2026. Nguồn duy nhất là
`crates/guinea-tui/src/commands.rs`: bộ điều phối phím, danh sách **F2**, phím
chuyển thẻ trong hộp thoại và các test tài liệu cùng đọc bảng này. Test bảo đảm:
không có hai thao tác trùng phím; phím chính của mỗi thao tác truyền được qua
terminal (không phải Ctrl+Shift+chữ, không phải Ctrl+M/I/[/J); hướng dẫn F1 Việt/Anh
nhắc đến mọi phím chính.

**F1** mở hướng dẫn Việt/Anh (cuộn bằng ↑↓, PageUp/PageDown, Home/End, kể cả ở
30×9). **F2** mở danh sách thao tác: gõ để lọc, mũi tên chọn, Enter chạy. Mỗi dòng
ghi phím tương ứng; thao tác đang bị khóa ghi kèm lý do (file chỉ đọc, AI đang
xử lý, thẻ xem trước, tác vụ nền).

## Chính sách xung đột phím

1. **Giữ phím đã có** của Guinea Note (Ctrl+L đến dòng, Ctrl+G tìm trong project,
   F6/F7/F8/F10…) để không phá thói quen hiện tại.
2. **Thao tác mới theo Notepad++** khi terminal truyền được: Ctrl+D nhân đôi,
   Ctrl+←/→ theo từ, Ctrl+Backspace/Delete xóa theo từ, Tab/Shift+Tab thụt khối,
   Ctrl+X/C không chọn = cả dòng. Di chuyển dòng dùng **Alt+↑/↓** (kiểu VS Code)
   vì Ctrl+Shift+↑/↓ của Notepad++ trùng phím cuộn của Windows Terminal.
3. **Mỗi thao tác có phím chính truyền được qua terminal**; tổ hợp dễ bị giữ chỉ là
   phím phụ. Mọi thao tác chạy được bằng tên trong F2.

## Thẻ và file

| Thao tác | Phím chính | Phím phụ |
| --- | --- | --- |
| Tạo thẻ | Ctrl+N | Ctrl+T |
| Mở file / thư mục | Ctrl+O | |
| Chọn thư mục project | Ctrl+K | |
| Lưu / lưu thành | Ctrl+S / **Alt+S** | Ctrl+Shift+S |
| Đóng thẻ / thoát | Ctrl+W / Ctrl+Q | |
| Quản lý toàn bộ thẻ | **F12** | Ctrl+P |
| Thẻ trước / sau | **F6 / F7** | Shift+F6; Alt+← / Alt+→; Ctrl+PageUp / Ctrl+PageDown; Ctrl+Shift+Tab / Ctrl+Tab |

Trong quản lý thẻ: gõ để lọc, Ctrl+U xóa lọc; ↑↓ hoặc Tab/Shift+Tab chọn; Enter
chuyển, Ctrl+S lưu, Ctrl+W đóng, Ctrl+N tạo; Esc/Ctrl+P/F12 đóng danh sách.

## Soạn thảo

| Thao tác | Phím chính | Phím phụ |
| --- | --- | --- |
| Hoàn tác / làm lại | Ctrl+Z / Ctrl+Y | Ctrl+Shift+Z |
| Chọn hết | Ctrl+A | |
| Sao chép / cắt / dán (không chọn: cả dòng) | Ctrl+C / Ctrl+X / Ctrl+V | |
| Từ trước / sau | Ctrl+← / Ctrl+→ | Alt+B / Alt+F |
| Chọn theo từ | Ctrl+Shift+← / Ctrl+Shift+→ | |
| Xóa từ phía trước / sau | Ctrl+Backspace / Ctrl+Delete | Alt+Backspace / Alt+D |
| Thụt / lùi lề (vùng chọn nhiều dòng) | Tab / Shift+Tab | |
| Nhân đôi dòng hoặc vùng chọn | Ctrl+D | |
| Đưa dòng lên / xuống | Alt+↑ / Alt+↓ | |
| Đầu / cuối tài liệu | Ctrl+Home / Ctrl+End | |
| Đến dòng[:cột grapheme], 1-based | Ctrl+L | |

Enter giữ thụt lề của dòng hiện tại (không áp dụng khi đang dán). Gõ liên tục là
một bước hoàn tác cho tới khi dừng khoảng 1 giây, di chuyển con trỏ, lưu hoặc dùng
lệnh khác. Chi tiết: [hành vi editor](EDITOR_BEHAVIOR.md).

## Một bảng Find/Replace

**Ctrl+F và Ctrl+H mở cùng một bảng**, luôn có ô tìm và ô thay. Trong bảng:

| Thao tác | Phím |
| --- | --- |
| Tới ô tìm / ô thay và chọn nội dung ô | Ctrl+F / Ctrl+H |
| Qua ô nhập, kết quả và các nút | **Tab / Shift+Tab** |
| Kích hoạt nút đang focus | **Enter / Space** |
| Kết quả sau / trước | F3 / Shift+F3; Enter / Shift+Enter ở ô nhập; ↓ / ↑ |
| Về vị trí kết quả trong editor | Enter ở danh sách hoặc nút Go |
| Thay một / tất cả | **Ctrl+R / Alt+A** (Ctrl+Shift+R nếu terminal phân biệt được) |
| Phân biệt hoa thường / khớp cả từ | **Alt+C / Alt+W** (bật/tắt; mặc định phân biệt hoa thường, không khớp cả từ) |
| Chọn hết / xóa ô | Ctrl+A / Ctrl+U |
| Đóng, hủy công việc chưa áp dụng | Esc |

Thứ tự Tab: **Tìm → Thay bằng → Kết quả → Trước → Sau → Go → Thay → Tất cả**.
Ctrl+F rồi Tab cũng tới ô thay nếu terminal không truyền đúng Ctrl+H.

## Không gian làm việc và xem

| Thao tác | Phím chính | Phím phụ |
| --- | --- | --- |
| Hướng dẫn / danh sách thao tác | F1 / F2 | |
| Tìm kết quả tiếp theo | F3 | |
| Chuyển soạn thảo / cây thư mục | F4 | |
| Đọc lại cây thư mục | F5 | |
| Chọn bố cục xem trước | F8 | |
| Thông tin file / thiết lập | F9 / F10 | |
| Chế độ tập trung | F11 | **Shift+F11** |
| Ẩn/hiện cây thư mục | Ctrl+B | F2 `explorer` |
| Tìm file / tìm nội dung project | Ctrl+E / Ctrl+G | |
| Mục lục Markdown | Alt+M | |

Cây thư mục: ↑↓ chọn, → mở nhánh, ← thu, Enter mở, Tab/Esc về editor. Chọn file:
↑↓/Page chọn, Enter mở, Tab điền, Alt+↑ thư mục cha, Ctrl+U xóa, Esc hủy. Xem
trước: mũi tên/Page/Home/End cuộn, Esc về nguồn. Xác nhận mặc định **Tiếp
tục/Keep working**; Tab/←→ chọn, Enter chạy, Y xác nhận bỏ, N/Esc hủy.

## Khi terminal giữ phím

Bảng dưới ghi các phím mặc định của terminal phổ biến có thể không tới được
Guinea Note, và cách thay thế. Đây là **phân tích theo cấu hình mặc định** của
từng terminal và cách mã hóa phím của crossterm; kiểm chứng runtime hiện chỉ chạy
trên Windows ConPTY (không qua Windows Terminal), nên chưa xác nhận từng dòng
trên từng terminal.

| Phím | Bị giữ hoặc đổi nghĩa ở | Dùng thay |
| --- | --- | --- |
| F11 | Windows Terminal, GNOME Terminal: toàn màn hình; macOS: Show Desktop | **Shift+F11**, F2 `focus-mode` |
| F10, F1 | GNOME Terminal: mở menu, Help | F2 `settings`, F2 `help` |
| Ctrl+Tab / Ctrl+Shift+Tab | Windows Terminal và terminal có thẻ: đổi thẻ terminal | F6 / F7, Alt+← / Alt+→ |
| Ctrl+PageUp / Ctrl+PageDown | GNOME Terminal, Konsole: đổi thẻ terminal | F6 / F7 |
| Alt+← → ↑ ↓ | Windows Terminal khi có nhiều pane (chuyển pane); macOS Terminal gửi Option+←/→ thành Alt+B/F | F6 / F7; F2 `move-line-up/down`; Alt+B/F vẫn là di chuyển theo từ |
| Ctrl+Shift+chữ (S, Z, R) | Mã hóa terminal cũ không phân biệt với Ctrl+chữ; Windows Terminal dành Ctrl+Shift+T/W/P/F/C/V | Alt+S, Ctrl+Y, Alt+A |
| Ctrl+Shift+↑ / ↓ | Windows Terminal: cuộn | Alt+↑ / Alt+↓ |
| Ctrl+B / Ctrl+A | tmux / GNU screen dùng làm prefix | F2 `explorer`, F4; F2 `select-all` |
| Ctrl+Backspace | Nhiều terminal Unix gửi `^H`, crossterm đọc thành Ctrl+H (mở Thay thế) | **Alt+Backspace** |
| Ctrl+C | Windows Terminal: sao chép khi đang có vùng chọn của terminal | F2 `copy` |
| Ctrl+V | Windows Terminal: tự dán (vẫn vào editor như một lần dán) | — |
| Ctrl+P, Ctrl+E, Ctrl+K, Ctrl+G | Terminal tích hợp của VS Code có thể giữ cho lệnh của VS Code | F12, F2 |
| Ctrl+M, Ctrl+I, Ctrl+[, Ctrl+J | Là Enter, Tab, Esc, LF | Không gán cho thao tác nào |

Ctrl+S, Ctrl+Q, Ctrl+Z và Ctrl+Y không bị hiểu thành XON/XOFF/tạm dừng vì Guinea
Note bật raw mode. Trên Windows, adapter đọc mã phím ảo nên phân biệt được
Ctrl+Backspace với Ctrl+H và Ctrl+Shift+chữ với Ctrl+chữ.

Ảnh bảng phím v1 (EN/VI, 30×9): [docs/diagnostics/keymap-v1](diagnostics/keymap-v1/manifest.json).
Ảnh trong `diagnostics/keyboard/` là bằng chứng lịch sử trước khi đổi F7/F8/F10.

Kiểm chứng Windows/ConPTY với `mouse = false`: `python -X utf8 scripts/pty_keyboard.py`.
Runtime Linux/macOS/SSH vẫn được ghi riêng theo phạm vi dự án.
