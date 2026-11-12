# Workflow terminal và project

Tập trung TUI cho terminal/SSH; không có GUI hoặc Node runtime. Header khi soạn
thảo thu về một hàng; welcome vẫn có nhãn sản phẩm. Nền navy tối được giữ,
chi tiết xanh lá, hổ phách và trắng ấm. F11 mở chế độ tập trung, giữ tab/count
và status; F11 trở lại workspace. F4/Ctrl+B mở lại cây và thoát focus mode.

Menu chính và toolbar quản lý tab có hàng phân cách riêng. Vùng click của menu,
nút quản lý tab và nhãn tab được kiểm tra tách biệt ở VI/EN và cửa sổ nhỏ;
focus mode vẫn dành phần lớn chiều cao cho nội dung.

## Điều hướng và khám phá lệnh

Xem [bảng phím tắt đầy đủ](KEYBOARD_SHORTCUTS.md). F1 cuộn được bằng ↑↓,
PageUp/PageDown, Home/End ở mọi kích thước hỗ trợ; F2 cho chọn thao tác bằng
mũi tên và Enter mà không cần gõ lệnh.

- Ctrl+E: tìm file trong project; các từ truy vấn phải xuất hiện trong đường dẫn.
- Ctrl+G: tìm literal, phân biệt hoa/thường trong **nội dung đã lưu trên đĩa**.
- Tab đổi Files/Text, Ctrl+U xóa truy vấn, F5 lập lại index; ↑↓/Page chọn,
  Enter hoặc double-click mở, Esc hủy. Chuột chọn/cuộn và các nút có keyboard fallback.
- F2 có danh sách lệnh VI/EN, lọc theo tên/mô tả; ↑↓ chọn, Tab hoàn tất, Enter chạy.
  Registry gợi ý vẫn gọi dispatcher hiện có; chưa thống nhất mọi disabled reason.
- Ctrl+L (hoặc F2 `goto`): nhập dòng[:cột grapheme] 1-based. Cột cuối hợp lệ là vị trí sau
  grapheme cuối. Dòng/cột 0, vượt phạm vi hoặc sai cú pháp bị từ chối, không sửa file.
- Ctrl+P/F12 quản lý mọi tab; F6/F7 hoặc Alt+→/← chuyển; Ctrl+T thêm tab.
  Trong danh sách thẻ, Tab/Shift+Tab hoặc ↑↓ chọn, Enter chuyển, Ctrl+S lưu,
  Ctrl+W đóng. Alt+S mở Lưu thành. Ctrl+F/H dùng một bảng Tìm/Thay thế chung;
  Tab tới cả hai ô và các nút, Enter kích hoạt, Ctrl+R thay một, Alt+A thay tất cả.

Project search không thay thế tìm kiếm buffer Ctrl+F/H. Nếu file đã đổi hoặc
buffer đang dirty sau khi search, mở tab nhưng không áp vị trí disk result vào
buffer; nội dung/caret hiện có được giữ và thông báo tìm lại trong tab.
Readonly/AI lease và save explicit không thay đổi.

## Giới hạn và hiệu suất

Index chạy nền, tối đa 20.000 files/40.000 entries/depth 32, charge 8 MiB;
hiển thị tối đa 200 kết quả. `.gitignore`/`.ignore` được tôn trọng trong root;
không đọc global/parent ignore, không đi theo symlink/reparse. `.git`, target,
node_modules, .venv và __pycache__ bị bỏ qua. File ẩn thông thường có thể hiện.
Đây là browsing của người dùng; không cấp thêm quyền đọc secret cho Agent.

Content scan tối đa 10 MiB/file và 256 MiB/lượt; chỉ strict UTF-8 được hỗ trợ.
File binary/không đọc được bị đếm skipped. Kết quả có cờ truncated khi chạm quota;
không được hiểu là toàn bộ project. Truy vấn tối đa 1024 UTF-8 bytes.

Một worker/panel, debounce 40 ms, latest-epoch publication, cancellation và chung
hai CPU permits với tác vụ review/search. Đóng panel không chờ scan hoàn tất.
OS read/enumeration đang block không bị ngắt tức thời. Index charge không phải
hard RSS bound; parser ignore và allocation hệ thống cần profile riêng.

## Terminal và nền tảng

`NO_COLOR`, `no_color`, `basic_colors` (256 màu), `ansi_colors` (16 màu), truecolor.
TERM linux/vt100/vt102/ansi chọn ANSI16 mặc định. Không bắt buộc Nerd Font.
`mouse=false` cho keyboard-only. Clipboard: `clipboard = "auto"` dùng bộ nhớ tạm
hệ thống khi chạy local (Windows API; pbcopy/wl-copy/xclip/xsel chưa kiểm chứng
runtime), bộ nhớ nội bộ khi qua SSH; `"osc52"` sao chép qua terminal (tối đa 74 KiB),
`"internal"` không chạm bộ nhớ tạm hệ thống. Probe đặt `GUINEANOTE_CLIPBOARD=internal`.

Windows legacy console adapter nhận LF và ghép Enter/LF liền kề trong cửa sổ
lookahead tối đa 2 ms để giữ CRLF ở fixture native. ConPTY legacy không luôn
giữ bracketed-paste delimiters: chưa có cam kết paste toàn khối là một Undo,
và CR/LF bị chia/trễ có thể mơ hồ. Không coi đây là chứng nhận mọi terminal paste.

`scripts/terminal_matrix.py` chạy 6 cấu hình local, hoặc `--tmux`, hoặc
`--ssh user@host --remote-binary /path/to/guinea` trên host được cung cấp rõ.
Không cài binary/service hoặc sửa SSH config. Fixture riêng, byte verify,
cleanup terminal và raw logs được giữ. Máy hiện tại chỉ kiểm chứng native
Windows/ConPTY; Linux/macOS/SSH/tmux chưa có runtime evidence.

GitHub Actions chưa kích hoạt. Build/release matrix native, artifact/checksum,
SBOM, package availability và install/rollback là release gates còn mở.
