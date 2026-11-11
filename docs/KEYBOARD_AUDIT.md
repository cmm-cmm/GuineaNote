# Kiểm chứng giao diện bằng bàn phím — 11/11/2026

Checkpoint lịch sử trước lần đổi function keys. Mapping hiện hành theo yêu cầu
người dùng: **F6 trước, F7 sau, F8 Review, F10 Settings, Alt+M mục lục**.
Xem [bảng phím hiện hành](KEYBOARD_SHORTCUTS.md); các số liệu và capture bên dưới
giữ nguyên theo phiên bản đã đo.

Theo yêu cầu người dùng, Find và Replace đã hợp nhất thành một bảng luôn có
cả hai ô nhập. Header và danh sách F2 chỉ còn một mục Find/Replace; Ctrl+F và
Ctrl+H đều mở bảng này. Alias lệnh cũ vẫn được giữ tương thích.

## Thay đổi đã triển khai

- Tab/Shift+Tab qua hai ô nhập, kết quả và năm nút search. Enter/Space kích hoạt
  nút đang focus; Ctrl+R thay một, Alt+A thay tất cả. Ctrl+Shift+R giữ làm alias.
- F12/Ctrl+P quản lý thẻ; F6/Shift+F6, Alt+→/← và Ctrl+PageDown/PageUp chuyển
  thẻ. Ctrl+Tab/BackTab được chuẩn hóa, kể cả khi BackTab không kèm SHIFT.
- Danh sách thẻ nhận Tab/Shift+Tab; toolbar rộng và status hiển thị gợi ý phím.
  Search/project cho chuyển thẻ an toàn; hộp lưu/xác nhận giữ ngữ cảnh riêng.
- Ctrl+L mở Đến dòng; Alt+S mở Lưu thành; Ctrl+U xóa ô nhập. Settings và chọn
  review nhận Shift+Tab. F2 dùng mũi tên/Enter mà không cần nhập tên lệnh.
- F1 Việt/Anh cuộn bằng mũi tên/Page/Home/End. Wrap giữ nguyên grapheme. Ở 30×9,
  cả hai ô, số kết quả, nút focus và gợi ý Esc có hàng riêng; nút focus được
  đưa vào vùng nhìn thấy khi chiều ngang hẹp.
- Alt+←/→ và F12 không đổi lựa chọn mặc định trong xác nhận bỏ dữ liệu.
  Adapter Win32 xử lý Alt+letter có Unicode unit bằng 0, đồng thời giữ AltGr.

[Bảng phím tắt đầy đủ](KEYBOARD_SHORTCUTS.md).

## Kiểm chứng

**146 Rust tests**, rustfmt, strict Clippy và workspace release build pass.
Test mới kiểm tra reverse-tab encoding, Unicode wrapping, focus/buttons ở
30×9–148×42, Help End/Home và bảo vệ xác nhận dirty. Hai test replacement cũ
giờ chờ worker permit dùng chung thay vì giả định luôn còn slot khi chạy song
song; giới hạn worker runtime không thay đổi.

12 native probes chạy tuần tự đều exit 0:

| Probe | Phạm vi |
| --- | --- |
| `pty_keyboard.py` | 8 nhóm workflow chỉ dùng phím, config `mouse = false`, không bật mouse capture |
| `pty_smoke.py` | Soạn/lưu, phím cơ bản, idle và cleanup |
| `pty_search.py --trace` | Unicode/BOM/CRLF, thay một/tất cả, Undo, readonly, 10 MiB |
| `pty_tabs.py` | Thẻ tràn, save/close, xác nhận dirty, review, VI/EN |
| `pty_dialog_mouse.py` | Hồi quy hit regions, hộp thoại, mouse và search |
| `pty_project.py`, `pty_workspace.py` | Mở/duyệt/tìm project và giữ buffer |
| `pty_agent.py`, `pty_recovery.py` | Lease/readonly Agent và recovery |
| `terminal_matrix.py` | Sáu trường hợp terminal, màu/kích thước và keyboard-only |
| `pty_interaction_latency.py`, `--no-trace` | Corpus tương tác 1 KiB/10 MiB, cùng binary |

Keyboard-only: 12 feedback samples, max **12.884 ms**. General trace-on:
**2.050 input-to-flush samples**, max **4.644 ms**, zero dropped. PTY max
trace-on/off **17.018 / 27.901 ms**. Không có mẫu tương tác >50 ms hoặc warning
trong hai lượt general này. Percentile theo từng nhóm, cold/warm và corpus
được giữ trong raw; không suy ra SLA toàn cục từ các sample hữu hạn.

## Evidence và giới hạn

Native Windows x64/MSVC release, ConPTY. Binary SHA256:
`16de88b190e42f2ae1a7663c25cdf1edb206e7ef28d55ca49c1d07eccd9af0c9`.
Source SHA256 theo collector:
`9c4f55067a98733313f0651257bc3d4051fd4310d142cd94653d5627d5fe0a76`.
Working tree trên dev `3cca7bf`; chưa commit/push.

- [Kết quả probes](diagnostics/keyboard/results.json),
  [keyboard report](diagnostics/keyboard/keyboard.json),
  [môi trường](diagnostics/keyboard/environment.json).
- [Trace on](diagnostics/keyboard/latency-trace.json),
  [trace off](diagnostics/keyboard/latency-off.json),
  [raw logs/CSV ZIP](diagnostics/keyboard/raw.zip),
  [manifest](diagnostics/keyboard/manifest.json).
- [Ảnh bảng chung](diagnostics/keyboard/keyboard-search-unified.png),
  [30×9](diagnostics/keyboard/keyboard-search-small.png),
  [F1 Việt](diagnostics/keyboard/keyboard-help-vi-end.png).
- Preliminary captures được giữ trong `preliminary.zip`, gồm lỗi test tranh
  permit và các correction của harness: chờ đúng trạng thái save hiện tại,
  tên hướng dẫn mới và tab active sau khi đóng. Assertions về bytes/cleanup
  được giữ; không hạ ngưỡng latency.

Ctrl+Tab được kiểm tra ở dispatcher/Win32 decoder; terminal có thể giữ tổ hợp
này, nên F6/Shift+F6 và Alt+←/→ là lựa chọn trực tiếp. Chưa chứng nhận runtime
Unix/SSH hoặc physical paint. Cold End lịch sử 916.326 ms và root cause chính
xác mẫu search lịch sử 55.417 ms tiếp tục thuộc các điều tra trước.
