# 05 — Ưu tiên tính năng và roadmap

Ngày: **10/11/2026**. Đây là backlog đề xuất, không phải các thay đổi đã triển khai. MVP hiện đã có nhiều chức năng vượt spec ban đầu theo yêu cầu người dùng; phân loại lại không có nghĩa xóa các chức năng đang có.

## Quy tắc ưu tiên và chi phí

- **P0:** nguy cơ mất dữ liệu, thao tác sai document/quyền, lỗi latency chưa giải thích và thiếu evidence chặn tuyên bố release.
- **P1:** khả năng soạn thảo cơ bản, hiệu suất/RAM ổn định và đường sử dụng trực quan.
- **P2:** tăng năng suất sau khi hợp đồng core và safety đủ vững.
- Chi phí **S/V/L/RL** lần lượt nhỏ/vừa/lớn/rất lớn, là đánh giá tương đối về implementation + tests + bảo trì; chưa phải dự toán ngày công. Những thứ có nền tảng hiện hữu có thể rẻ hơn nhưng vẫn cần regression.

Nguồn phân loại: [khảo sát editor](01_EDITOR_COMPARATIVE_RESEARCH.md), [engine](02_TEXT_ENGINE_ARCHITECTURE.md), [render và benchmark](03_TUI_RENDERING_PERFORMANCE.md), [file safety](04_FILE_SAFETY_AND_UNICODE.md), code tại commit nền đã ghi.

## MVP bắt buộc

| Tính năng / hạng mục | Hiện trạng đã xem | Ưu tiên / chi phí | Dependency | Acceptance criteria đề xuất |
| --- | --- | --- | --- | --- |
| Open/save/save-as an toàn | Có strict UTF-8/BOM, fingerprint, temp replacement | P0 / L | Core snapshot + I/O | Byte goldens; failure trước commit không đổi original; outcome sau commit đúng |
| Caret/selection/edit Unicode | Có grapheme và cells riêng | P0 / L | Coordinates + buffer | NFC/NFD/CJK/ZWJ/tabs/CRLF và boundary property tests, mouse cùng policy |
| Undo/Redo và dirty | Snapshot/history, state token | P0 / V | Transaction | Save không xóa Undo; Undo về savepoint clean; branching đúng; không false-clean |
| Điều tra latency và quota | Có probes, còn outlier 916 ms | P0 / L | Trace/benchmark corpus | Giữ raw failure; trace/repro hoặc ghi chưa rõ; gate <100 ms trên corpus công bố |
| Tabs | Có new/save/close/count/switch/manager | P1 / V | Stable tab/document identity | Không nhầm active/inactive tab; dirty confirm; keyboard + mouse; overflow dùng được |
| File explorer và picker | Có lazy listing và lọc | P1 / V | Worker generation + path policy | Không recursive startup; không follow symlink ngầm; kết quả cũ không áp root mới |
| Find/replace literal | Panel trực quan, worker/page, một Undo | P1 / V | Snapshot/revision/access checks | Kết quả Unicode đúng; readonly/AI không mutate; replace-all atomic logic, save riêng |
| Go to line | Chưa thấy command trong command list hiện tại | P1 / S–V | Exact line index + view | Dòng 1-based, báo out-of-range; không ước lượng số dòng lớn thành vị trí thật |
| Command palette cơ bản | F2 command prompt có fallback | P1 / V | Một action registry | Filter tên VI/EN, phím và disabled reason thống nhất; mọi thao tác chính truy cập được |
| Syntax highlighting cơ bản | Nhiều nhãn/ngôn ngữ, lexer line-local | P1 / V | Revision spans + viewport | Không phá source; không nhận diện sai vì clip window; nói rõ grammar chưa đầy đủ |
| Clipboard nội bộ | Có | P1 / S | Selection + paste transaction | Byte/text Unicode đúng; paste một Undo; không bắt có helper ngoài |
| Status bar / file details | Có; format ở phải | P1 / S | Metadata async | Dirty/readonly/AI/position dễ thấy; số liệu có phạm vi/revision, không cắt tên file mất nghĩa |
| Theme và VI/EN | Có navy/xanh/hổ phách/trắng, RGB/256 | P1 / V | Capability policy + i18n | Contrast đủ, không chỉ dựa màu; hẹp vẫn dùng được; xác minh/sửa NO_COLOR lệch tài liệu |
| Shortcuts cơ bản + cấu hình tối thiểu | Phím hiện hard-code, F2 fallback | P1 / V | Command registry | Giữ phím quen; tài liệu phím đúng; chưa cần keymap editor đa tầng |
| Mouse/resize/terminal cleanup | Có native Windows probes | P1 / V–L | Input adapter + hit regions | Modal không click xuyên; drag/wheel đúng; resize không stale hit; native ba OS |

Nếu mục tiêu first release là chỉ Windows trước, phải ghi nhãn platform support tương ứng; không đánh dấu Linux/macOS supported chỉ vì cross-Clippy pass. Không mở rộng feature mới để né hardening.

## Sau MVP

| Tính năng | Lý do / lợi ích | Chi phí / phụ thuộc | Acceptance criteria |
| --- | --- | --- | --- |
| Recovery drafts + backup tùy chọn | Bảo vệ edit chưa lưu; ưu tiên đầu nhóm này | L / Safe save, snapshot/quota | Kill/corrupt/base-conflict tests; restart thành draft; không tự overwrite source |
| Session restore | Mở lại tabs/caret/layout tiện hơn | V / Schema version, recovery cho dirty | Không biến missing/external-changed file thành mất draft; migration config có backup |
| Split view chỉnh sửa | So sánh hai vùng cùng file hoặc hai file | L / Document–View tách ownership | Một history/savepoint, caret riêng; edit cập nhật mọi view; close view không xóa document |
| Keybindings cấu hình đầy đủ | Thích nghi terminal và thói quen | V–L / Action registry, capability fallback | Detect xung đột, default/reset, load lỗi không mất đường Help/Settings |
| System clipboard + OSC 52 | Copy/paste local và SSH | V / Provider policy, terminal timeout | Internal fallback; không chặn UI, không log secrets; test SSH/tmux cụ thể |
| Regex/case/whole-word/selection search | Tìm kiếm code mạnh hơn | L / Matcher API, Unicode mapping | Không split grapheme sai; semantics rõ; zero-length match tiến được; output cap/cancel |
| Tree-sitter theo nhu cầu | Syntax state chính xác hơn lexer cơ bản | L / Grammar license/version, snapshots | Stale spans bị loại; parser errors không đổi file; đo grammar load/RSS/long line |
| Indentation, comment toggle, bracket match | Tăng năng suất code với scope nhỏ | V / Language settings + transaction | Một Undo/selection mapping đúng; không bắt mọi language rule vào core |
| MD/data review tiếp tục cải thiện | Đã có split/tab review theo yêu cầu | V–L / Bounded parser/cache, source mapping | Source không bị serialize lại; invalid input còn xem source; cap bytes/CPU/depth rõ |
| Workspace search | Tìm trong nhiều file hữu ích hơn plugin sớm | L / Traversal/ignore/secret policy, job scheduler | Cancel, báo skipped/errors; limit results; không đọc .env trái Agent policy |

Split **review** hiện đã có, nên duy trì như tính năng đang dùng; không gọi nó là split editable hoàn chỉnh. MD review và data review không phải “format file”: chúng không được thay đổi bytes nguồn khi xem.

## Nâng cao

| Tính năng | Giá trị và chi phí bảo trì | Điều kiện bắt đầu |
| --- | --- | --- |
| Multiple selections / rectangular selection | Năng suất cao nhưng lan sang movement, Undo, replace và Unicode/cells | Transaction mapping, view model và overlap semantics ổn định; L |
| LSP client | Diagnostics/completion/rename nhiều ngôn ngữ; thêm server lifecycle, root/config và protocol errors | MVP ổn định, bounded Language Services; RL |
| Paged read-only 100 MB–1 GB | Giữ working set hữu hạn | Prototype thắng đo RAM/I/O và exact-index contract; L |
| Paged editable | File lớn có Undo/safe save/snapshot isolation | Read-only pass, recovery và immutable base store; RL |
| Plugin API/runtime | Tùy biến, nhưng cần versioning, quyền và lỗi cách ly | Có nhu cầu cụ thể không đáp ứng bằng commands/MCP; RL |
| Formatter external opt-in | Có thể tận dụng formatter chuẩn từng ngôn ngữ | Workspace trust, preview, timeout, một Undo, explicit invocation; L |
| Macro/snippets, folding, diagnostics panel | Chỉ thêm khi task thực tế cần | Action semantics, grammar và view model tương ứng; V–L |

Không hứa “mọi ngôn ngữ đều có LSP/highlight chính xác”. Mở mọi file UTF-8 như văn bản, highlight generic và hỗ trợ semantic đầy đủ là ba mức capability khác nhau. Với `.env` và config cần giữ bytes và secrets; không tự đánh giá nội dung như shell.

## Không nên triển khai theo định hướng hiện tại

| Hạng mục | Lý do |
| --- | --- |
| Chuyển toàn app sang backend mới chỉ vì mới hơn | Chưa có evidence, tăng rủi ro Unicode/history/save |
| Embedded Node.js làm runtime editor | Trái yêu cầu native độc lập runtime ngoài |
| Auto-run code/plugin/modeline/shell từ project khi mở file | Không cần cho text editing, tăng rủi ro và support burden |
| Auto-format/normalize/convert encoding khi mở hoặc save mặc định | Có thể âm thầm thay bytes và semantics |
| Writable mmap lên original file | Không có safe insert/delete/save contract phù hợp |
| Tự viết renderer terminal thay Ratatui trước profiling | Tạo thêm Unicode/capability/resize bugs |
| Quản lý DB live, notebook kernel, debugger/build IDE trọn bộ ở MVP | Ngoài scope văn bản/code, nhiều runtime và vòng đời lỗi |
| Plugin ABI native không version/quyền, arbitrary dynamic loading | Khó cách ly lỗi và giữ MIT/dependency policy |
| Hiện số dòng file lớn ước lượng như chính xác | Trái nguyên tắc không đổi correctness lấy latency |
| Auto-update background tự thay executable lúc đang edit | Không cần; package manager và versioned release dễ rollback hơn |

## Roadmap theo milestone

### R0 — Baseline nghiên cứu và quyết định

Đầu ra: bộ tám tài liệu này, commit nền, nguồn và giới hạn. Không đổi source, dependency, branch, CI, package hoặc commit. **Hoàn thành khi** mọi đề xuất có priority/evidence và kiểm tra phạm vi file chỉ trong `docs/research/`.

### R1 — Quan sát và budget hiệu suất

Dependency: R0. Instrument latency spans, cold End, clone/history/drop, frame/flush; corpus long-line và workload đồng thời. Chuẩn hóa p95/p99/max và raw output; đo peak RSS/open/save/idle. **Cổng:** không bỏ outlier; workload chưa pass phải còn hiện là failure. Không tuyên bố đã sửa 916 ms nếu chỉ không tái hiện.

### R2 — Safety và working set

Dependency: R1 và semantic suite hiện có. Snapshot không kèm history, streaming save/open có strict validation, quota history/cache/output, cancellation trước commit và metadata/outcome policy. **Cổng:** byte goldens, save khi đang edit, failure/kill và conflict tests; native Windows trước, rồi Linux/macOS. Peak RSS và UI không regression; quota không âm thầm xóa Undo/draft không thông báo.

### R3 — UX nhất quán và tính năng cơ bản còn thiếu

Dependency: transaction/snapshot contract R2; có thể làm thiết kế action registry sau R1. Thêm Go to line, palette, syntax context đúng, capability/NO_COLOR, shortcuts fallback; giữ dialog/mouse/tab/review hiện có. **Cổng:** keyboard và mouse đi cùng action checks, VI/EN đầy đủ, terminal hẹp/resize/readonly/AI khóa đúng, đo corpus <100 ms theo protocol.

### R4 — Release nền tảng

Dependency: R2/R3 và native evidence. Chuẩn hóa CLI `guinote`, chốt release support matrix, public source/artifact policy và tên package; build/test/release/checksum/signature ở giai đoạn phát hành. **Cổng:** native smoke + Unicode save/reopen trên từng target trước khi phát hành target đó; install/upgrade/rollback giữ config/drafts. Actions chỉ là thiết kế đến khi bước triển khai sau được giao; hiện không tạo CI.

### R5 — Recovery, session và split

Dependency: R2; có thể ưu tiên recovery trước R4 nếu phân phối cho người dùng cần draft protection. Recovery snapshot đầy đủ trước journal; session restore, view state riêng, editable split; clipboard provider và keymap theo nhu cầu. **Cổng:** crash/corrupt/mismatch không overwrite source; một document nhiều view dùng một lịch sử.

### R6 — File lớn có kiểm chứng

Dependency: R1/R2/R5. Prototype paged read-only, exact indexing tiến dần, bounded cache và immutable backing version. **Cổng:** benchmark 100/500/1.000 MB, sparse/long lines, cold I/O; Unicode và position correctness không giảm. Chỉ sau đó quyết định paged editable hay tiếp tục Ropey với giới hạn rõ.

### R7 — Language Services và mở rộng có chọn lọc

Dependency: action/snapshot/view model ổn định. Tree-sitter, regex mạnh hơn, multi-selection, LSP từng ngôn ngữ; plugin chỉ khi có use case vượt service/Agent hiện có. **Cổng:** optional services không giữ UI, revision/UTF-16 mapping đúng, shutdown/cancel sạch, grammar/dependency license đã audit.

## Quản lý thay đổi và bằng chứng

Mỗi milestone có PR/task riêng sau giai đoạn nghiên cứu, với baseline, expected behavior, tests và benchmark liên quan. Không gộp rename CLI, backend migration, file save rewrite và plugin system trong một thay đổi. Estimates ngày công chỉ lập sau profiling/spike; không làm deadline giả dựa trên số tính năng.

Các tính năng Agent/MCP đã có tiếp tục opt-in, source/linked review có AI badge và readonly khi lease active, tab khác còn dùng được. Không tự gỡ phần đã được người dùng yêu cầu, cũng không thêm public network Agent server hoặc capability mới trong nghiên cứu này.
