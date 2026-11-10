# 01 — Nghiên cứu so sánh các editor

Ngày nghiên cứu: **10/11/2026**. Đối tượng GuineaNote: commit `3186ddf49e181d14f465f29af1e8a6e7e4392383`, nhánh `dev`. Đây là nghiên cứu, không phải kết quả chạy benchmark các editor bên ngoài.

## Phương pháp và giới hạn

Đọc repository và tài liệu GuineaNote trước; sau đó đọc tài liệu chính thức và các phần mã nguồn liên quan của editor tham khảo. Mỗi phần phân biệt **đã xác minh** với **bài học đề xuất**. Không build, cài đặt hoặc đo hiệu năng Geany, Kate, gedit, Fresh, Nano, Micro, Helix hay Neovim trong đợt này. Những mô tả như “nhẹ”, “nhanh” trong giới thiệu sản phẩm không được coi là số đo so sánh.

Các commit nguồn, phiên bản tài liệu và giới hạn truy cập nằm trong [danh mục nguồn](08_RESEARCH_SOURCES.md). Không sao chép mã nguồn của các editor vào GuineaNote. So sánh GUI/TUI nhằm học cách tổ chức document, lịch sử, tác vụ và UX; không dùng chênh lệch RAM giữa một IDE GUI và editor TUI để kết luận về ngôn ngữ lập trình.

## Bức tranh tổng thể

| Editor | Cơ chế kỹ thuật đã kiểm tra | Giá trị áp dụng | Giới hạn đối với GuineaNote |
| --- | --- | --- | --- |
| Geany | GTK, Scintilla; tách dialog tìm kiếm khỏi thao tác document | Ít bước thao tác, phạm vi tìm kiếm rõ | Không mang cả IDE/build system vào MVP |
| Kate | KTextEditor, quản lý document/view riêng; session; LSP client plugin | Nhiều view dùng chung một document | Hệ sinh thái Qt/KDE không phù hợp để nhúng vào TUI |
| gedit | API tách Document/View/Tab; tính năng nâng cao có thể bật thêm | Progressive disclosure, tab có trạng thái | Không suy ra buffer nội bộ từ API công khai |
| Fresh | Rust TUI; Piece Tree có dữ liệu loaded/unloaded và snapshot chia sẻ | Lazy loading và UX quen thuộc | Cần xem kỹ độ chính xác số dòng trong chế độ lớn; GPL |
| GNU Nano | Modeless, help theo ngữ cảnh, prompt và file browser | Dễ khám phá và có đường bàn phím | Không sao chép nguyên bộ phím; tài liệu không chứng minh backend |
| Micro | Go TUI, LineArray, TextEvent Undo; clipboard theo provider; Lua | Non-modal, fallback khi terminal hạn chế | Line array có chi phí theo dòng; plugin làm tăng bề mặt bảo trì |
| Helix | Rust, Ropey, transaction/change mapping, render theo view | Unicode và transaction nhất quán | Modal UX không khớp định hướng; giấy phép snapshot là MPL-2.0 |
| Neovim | C/libuv, hàng đợi sự kiện; memline/memfile theo block; giao thức grid | Tách document, view và output; phục hồi là subsystem | Không tái tạo hệ plugin/RPC đầy đủ ở MVP |

Các nhận định ở bảng được chứng minh trong từng mục dưới đây; cột giá trị/giới hạn là đánh giá cho GuineaNote.

## Geany: giữ cấu trúc thao tác đơn giản

**Đã xác minh:** README nêu GTK và Scintilla. `src/search.c` phụ trách dialog Find/Replace/Find in Files, còn thao tác tìm kiếm văn bản cơ bản nằm trong `document.c`. Các response riêng phân biệt tìm tiếp, thay hiện tại, thay trong selection, file và session. Điều này giúp UX và thao tác document dùng chung khái niệm phạm vi thay vì giấu mọi chức năng trong một hộp thoại chung. Nguồn: [README](https://github.com/geany/geany/blob/0e02cc52a7634d1fed8511bd44730ea0ab791d9f/README), [search.c](https://github.com/geany/geany/blob/0e02cc52a7634d1fed8511bd44730ea0ab791d9f/src/search.c), [document.c](https://github.com/geany/geany/blob/0e02cc52a7634d1fed8511bd44730ea0ab791d9f/src/document.c).

**Bài học đề xuất:** giữ một model tìm kiếm độc lập dialog; biểu diễn phạm vi `Document / Selection / Workspace` rõ ràng. MVP tiếp tục chỉ thay trong document hiện tại; workspace replace chỉ thêm khi có preview, cancellation và báo cáo lỗi từng file. Không dùng hành vi ghi đè hàng loạt để mở rộng nhanh chức năng.

**Điểm mạnh/yếu cho bài toán:** ít phụ thuộc môi trường desktop so với một IDE đầy đủ là mục tiêu của Geany, nhưng bản thân nó vẫn cần GTK. Không có bằng chứng trong nghiên cứu này cho một ngưỡng file hoặc latency cụ thể. GuineaNote nên học cách chia chức năng, không dùng danh tiếng “lightweight” làm acceptance criterion.

## Kate: document không phải view

**Đã xác minh:** `KateMainWindow` phối hợp document manager và view manager. Bên dưới, KTextEditor `TextBuffer` quản lý các `TextBlock`; block chứa các dòng và liên kết với cursor/range. Vì vậy không nên mô tả Kate đơn giản là “dùng Ropey” hay suy đoán dùng cùng engine với GuineaNote. Nguồn: [KateMainWindow](https://github.com/KDE/kate/blob/d6977a183c3badc8c4ed8ef45c29a6db4770d006/apps/lib/katemainwindow.cpp), [TextBuffer](https://github.com/KDE/ktexteditor/blob/82325fbff097abea15c54c7ffcf2b3882fec79e5/src/buffer/katetextbuffer.cpp), [TextBlock](https://github.com/KDE/ktexteditor/blob/82325fbff097abea15c54c7ffcf2b3882fec79e5/src/buffer/katetextblock.h).

Session là một khái niệm có menu quản lý riêng. LSP client là plugin; tài liệu mô tả một server theo cặp root và loại server, hỗ trợ đồng bộ incremental. Đây là quản lý vòng đời tiến trình và workspace, không chỉ bật completion. Nguồn: [Sessions](https://docs.kde.org/stable_kf6/en/kate/kate/sessions-menu.html), [LSP client](https://docs.kde.org/stable_kf6/en/kate/kate/kate-application-plugin-lspclient.html).

**Bài học đề xuất:** khi thêm split chỉnh sửa thật, một `DocumentId` phải sở hữu một nội dung/lịch sử/savepoint; mỗi `ViewId` có caret, selection, scroll riêng. Linked review hiện có là bước hợp lý, chưa tương đương hai editable views. Session restore chỉ lưu view/layout và tham chiếu file; bản nháp chưa lưu cần recovery riêng. LSP về sau cần root policy, timeout và trạng thái server; không đặt trực tiếp trong widget.

**Chi phí:** chuyển selection đang nằm trong `Document` sang view là thay đổi liên quan API Agent, Undo và input. Chỉ làm theo milestone có test, không viết lại toàn bộ hiện tại. Kate cũng cho thấy feature phong phú kéo theo nhiều cấu hình, tiến trình ngoài và trạng thái lỗi; GuineaNote nên trì hoãn phần đó.

## gedit: đơn giản ở bề mặt, phân lớp ở bên trong

**Đã xác minh:** tài liệu API gedit **51.0** có `GeditDocument`, `GeditView`, `GeditTab` riêng, cùng interface extension ở app/window/view. `GeditTab` công khai state, API load file/stream và cấu hình autosave. Trang tính năng nêu tab, search/replace, file browser và plugin. Nguồn: [API 51.0](https://gedit-text-editor.org/developer-docs/gedit/), [GeditTab](https://gedit-text-editor.org/developer-docs/gedit/GeditTab.html), [GeditDocument](https://gedit-text-editor.org/developer-docs/gedit/GeditDocument.html), [GeditView](https://gedit-text-editor.org/developer-docs/gedit/GeditView.html), [tính năng](https://gedit-text-editor.org/features.html).

**Bài học đề xuất:** biểu diễn trạng thái `Opening / Ready / Saving / Conflict / Failed` có hành động phục hồi rõ. F8 hiện chỉ có ngôn ngữ là hợp lý; tránh đưa toàn bộ lựa chọn chuyên gia lên màn hình đầu. File details và command palette là cửa vào các khả năng nâng cao khi người dùng cần.

**Giới hạn chứng cứ:** URL GNOME cũ gặp lỗi truy cập; website chính thức hiện trỏ tới `World/gedit/gedit`. Không lấy được commit source gedit tin cậy qua các endpoint đã thử, nên không kết luận cấu trúc buffer, độ phức tạp hay cơ chế atomic save nội bộ. Cũng không mặc định gedit hiện tại dùng GtkSourceView 5: danh mục developer chính thức có các thư viện `libgedit-*` riêng. Đây là ví dụ vì sao cần kiểm tra phiên bản.

## Fresh: file lớn cần một chế độ vận hành có hợp đồng

**Đã xác minh ở source:** workspace Rust, runtime terminal có Ratatui/Crossterm. `piece_tree.rs` phân biệt `BufferData::Loaded` với `Unloaded`, dùng dữ liệu chia sẻ qua `Arc`, line index có thể vắng mặt. Buffer model giữ Piece Tree và cấu hình threshold. Nguồn: [workspace](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/Cargo.toml), [Piece Tree](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor-core/src/model/piece_tree.rs), [buffer](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor-core/src/model/buffer/mod.rs).

**Thông tin tác giả, không phải benchmark độc lập:** bài kỹ thuật tháng 12/2025 mô tả lazy loading và chế độ file lớn dùng byte navigation, ước lượng số dòng. Ngưỡng nêu trong bài là lịch sử, không được xem là hằng số của commit 2026 đang đọc. Nguồn: [bài viết của tác giả](https://noamlewis.com/blog/2025/12/09/how-fresh-loads-huge-files-fast).

**Bài học đề xuất:** backend lớn phải phân biệt khả năng đọc theo byte, đếm dòng chính xác và chỉnh sửa. GuineaNote không được hiển thị số dòng ước lượng như số chính xác. Có thể báo “đang lập chỉ mục” hoặc vô hiệu Go to line cho vùng chưa biết; không trả một vị trí sai để đạt mục tiêu latency. Snapshot chia sẻ và tải dữ liệu theo nhu cầu đáng học; không đủ căn cứ đổi Ropey ngay.

**Điểm mạnh/yếu:** Fresh rất sát bài toán UX TUI hiện đại và lazy file; nhưng scope đã lớn, có subsystem plugin/runtime riêng. Giấy phép workspace là `GPL-3.0-or-later`; chỉ tham khảo thiết kế, không chuyển code sang MIT. Không nhận các tuyên bố multi-gigabyte/low-latency trong README làm kết quả GuineaNote.

## GNU Nano: người dùng phải tìm thấy thao tác

**Đã xác minh:** manual **9.3** mô tả editor modeless, vùng thông báo/prompt, help luôn thấy, file browser, tìm/thay và hỗ trợ mouse. Khi wheel, viewport cuộn thay vì caret. Search có phạm vi selection; Undo/Redo, backup và locking là các khả năng có giải thích riêng. Nguồn: [manual Nano 9.3](https://www.nano-editor.org/dist/v9/nano.html).

**Bài học đề xuất:** duy trì help theo context, thông báo có bước tiếp theo, file picker và đường bàn phím cho mọi nút. Giữ Ctrl+F/Ctrl+H quen thuộc của GuineaNote nhưng cần fallback F2 vì control sequences có thể trùng trong terminal. Không sao chép nguyên bộ phím hay hành vi chuyển đổi newline của ứng dụng khác. Manual là bằng chứng UX; nghiên cứu này không xác nhận data structure nội bộ của Nano.

**Chi phí:** context help và keymap thống nhất thấp hơn nhiều so với thêm một chế độ modal. Đây là bài học phù hợp nhất với định hướng “dễ như Nano”. Hạn chế là vùng help dài cạnh tranh chỗ với nội dung ở cửa sổ nhỏ; cần layout responsive.

## Micro: non-modal và provider có fallback

**Đã xác minh:** repo người dùng đưa đã chuyển từ `zyedidia/micro` sang `micro-editor/micro`. Source hiện tại viết Go. `LineArray` giữ danh sách dòng; `TextEvent`/`Delta` ghi thao tác để Undo, có logic nhóm sự kiện theo thời gian. Clipboard terminal gọi provider tcell và có timeout khi đọc; Lua plugin có API/tài liệu riêng. Nguồn: [LineArray](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/internal/buffer/line_array.go), [history](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/internal/buffer/eventhandler.go), [clipboard](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/internal/clipboard/terminal.go), [plugins](https://github.com/micro-editor/micro/blob/ab37da303ae719a469236697239a00e3e1ca8b16/runtime/help/plugins.md).

**Bài học đề xuất:** Undo nên có nhóm typing/paste/replace rõ; clipboard tách internal/local/terminal với lỗi và fallback. Không để clipboard request chặn UI vô thời hạn. Line array đơn giản nhưng thêm/xóa nhiều dòng hoặc dòng dài có thể yêu cầu dịch/copy dữ liệu; không có benchmark ở đây chứng minh ưu/nhược bằng hệ số.

**Chi phí:** Lua giảm công xây tính năng tùy biến nhưng mở thêm hợp đồng API, tương thích phiên bản và rủi ro chạy code. GuineaNote đã có service/Agent protocol; nên ổn định command API trước một plugin runtime. MIT của Micro không có nghĩa phải hoặc nên port code Go sang Rust.

## Helix: transaction, Unicode và view rendering

**Đã xác minh:** `helix-core` re-export Ropey; `Transaction`/`ChangeSet` biểu diễn retain/delete/insert và mapping vị trí. Renderer lấy viewport và syntax highlighter theo view. Nguồn: [core](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-core/src/lib.rs), [transaction](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-core/src/transaction.rs), [editor renderer](https://github.com/helix-editor/helix/blob/ba40e547426b0f9896c8bdc699a4ab11f2b37dbc/helix-term/src/ui/editor.rs).

**Bài học đề xuất:** một transaction thống nhất cho người dùng và Agent; mapping selection, highlight và view qua edit là nền móng cho multi-selection/LSP. Trước mắt giữ API prepared edit + expected revision của GuineaNote. Bổ sung edit delta nhỏ có kiểm thử khi cache cần nó; chưa cần biểu thức change algebra đầy đủ.

**Điểm cần tránh:** UX modal/selection-first không phù hợp mục tiêu non-modal. Ropey được một editor khác dùng là bằng chứng khả thi, không phải chứng minh đủ nhanh với mọi layer của GuineaNote. LICENSE tại commit đã xem là **MPL-2.0**, không ghi GPL theo ký ức hoặc nguồn cũ; vẫn không được đổi mã MPL thành MIT bằng cách chép vào repo.

## Neovim: document, event loop và recovery là ba trách nhiệm

**Đã xác minh:** `memline.c` mô tả cây pointer/data blocks trên `memfile`, có block dành cho recovery; dòng vượt một page có thể dùng block nhiều page. `event/loop.c` có libuv, fast/thread/main queues và chống re-enter `uv_run`. Giao diện UI có `grid_line`, `grid_scroll`, `flush`. Nguồn: [memline](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/src/nvim/memline.c), [loop](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/src/nvim/event/loop.c), [UI events](https://github.com/neovim/neovim/blob/d175a05e0d24b2796f9624f8b6965d690394116e/src/nvim/api/ui_events.in.h).

**Bài học đề xuất:** worker gửi kết quả; main loop điều phối commit và render. Queue cần ưu tiên input, giới hạn dung lượng và tránh starvation. Crash recovery cần format, checkpoint, replay và kiểm thử riêng; temp-file safe save không thay thế nó. Không sao chép memfile: chi phí trưởng thành của subsystem lớn hơn nhu cầu MVP.

**Giới hạn:** native plugin/Lua/RPC của Neovim là một nền tảng mở rộng lâu năm; không phải danh sách cần làm cho GuineaNote v0.1. LICENSE có Apache-2.0 và phần Vim license; không đơn giản hóa thành MIT.

## Các kết luận áp dụng

1. Giữ non-modal, help dễ thấy và command fallback: Nano/Micro.
2. Một nội dung, nhiều view; đừng tạo hai bản mutable độc lập cho split: Kate/gedit.
3. Một transaction có revision và Undo cho mọi nguồn edit: Helix.
4. Dữ liệu bất biến và worker có vòng đời rõ: Helix/Fresh/Neovim.
5. Lazy loading cần công khai capabilities và tính chính xác: Fresh.
6. Search scope, loading/error và clipboard provider phải có model riêng: Geany/gedit/Micro.
7. Khả năng mở rộng chỉ thêm sau khi command/document contract ổn định.

Thứ tự triển khai cho GuineaNote nằm trong [roadmap](05_FEATURE_PRIORITY_ROADMAP.md); quyết định kiến trúc và rủi ro nằm trong [bản tổng hợp](07_GUINEANOTE_ARCHITECTURE_RECOMMENDATIONS.md).
