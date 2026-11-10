# GuineaNote — Triển khai kiểm soát năm rủi ro kiến trúc

Ngày: 10/11/2026. Người dùng đã yêu cầu triển khai sau giai đoạn nghiên cứu.
Baseline source: dev, 3186ddf49e181d14f465f29af1e8a6e7e4392383.
Checkpoint triển khai được ghi nhận trên nhánh dev; revision là commit chứa tài liệu này.
Người dùng đã cho phép commit/push checkpoint cùng tài liệu và evidence.
Không tạo GitHub CI, không publish package và không thay đổi nhánh main.
Tám báo cáo nghiên cứu và evidence thất bại cũ được giữ nguyên để đối chiếu.

## Phạm vi hoàn thành và mức bảo đảm

Đã triển khai kiểm soát cho cả năm nhóm; không đồng nghĩa xóa mọi rủi ro trên mọi
máy/filesystem. Windows x64 là môi trường chạy native. Linux x64/macOS ARM64 chỉ
được kiểm tra biên dịch/lint. Mốc <100 ms vẫn là cổng đo trên corpus <=10 MiB,
không phải bảo đảm thời gian thực cứng của terminal/OS.

| Nhóm rủi ro | Kiểm soát đã triển khai | Giới hạn còn lại |
| --- | --- | --- |
| 1. Cold latency 916.326 ms | Trace opt-in có timestamp/stage/drop count; giữ raw failure; mở rộng probe sang dòng Unicode dài thứ hai; cache giữ các dòng không bị sửa | Chưa xác định nguyên nhân mẫu 916.326 ms lịch sử; chưa có scheduler/physical-paint trace của lần đó |
| 2. RAM/history/copies/workers | Snapshot không history/cache; opaque content snapshot; stream open/save; quota Undo/Redo, cache và CPU jobs; bỏ delivery review tab ẩn | RAM của Ropey/DOM, tab đang mở và active working set vẫn phụ thuộc dữ liệu; không phải backend file lớn hay hard RSS cap |
| 3. Identity/metadata/race | Handle + identity + SHA-256; revalidate; từ chối hardlink/ADS path/link; Windows ADS và DACL, Unix ownership/xattr; temp private trước khi ghi | Không có filesystem CAS; vẫn có final-check→rename/ancestor race; SACL và mọi metadata đặc biệt chưa được bảo đảm |
| 4. Mất bản nháp khi crash | Recovery private/version/checksum/quota; OS session locks; bản nháp độc lập; handoff và cleanup có chủ đích; giữ provenance cho AI policy | Có thể mất edit sau checkpoint cuối; chưa chứng minh power-loss/disk-full/network FS; không thay thế backup |
| 5. Coupling và syntax clip | ContentSnapshot opaque; ViewState riêng; snapshot worker không clone histories; syntax từ line origin rồi clip, fallback plain khi context quá lớn | Document vẫn giữ active view tương thích; chưa có editable splits/DocumentStore hay parser-neutral crate hoàn chỉnh |

## Những thay đổi có thể quan sát

- Recovery bật mặc định, không tự lưu vào source. Bản nháp được khôi phục mở thành
  tab chưa lưu; Ctrl+S yêu cầu đường dẫn mới. F9 giữ thông tin nguồn khôi phục.
- Recovery lỗi có cảnh báo trên thanh trạng thái; lưu thủ công vẫn hoạt động.
- Undo/Redo giữ tối đa 2048 transaction và 64 MiB charge mặc định; luôn giữ bước
  gần nhất kể cả vượt quota. Khi cắt lịch sử cũ có thông báo; F9 cho biết số bước.
- Source rất dài không bị tô màu từ context sai: dùng prefix tối đa 16384 scalar,
  vượt ngưỡng hiển thị plain. Nội dung vẫn đầy đủ. NO_COLOR đã được kiểm thử.
- Nền navy, màu xanh lá/hổ phách/trắng và thao tác VI/EN được giữ lại.

## Quyết định và trade-off

Giữ Rust/Ropey/Ratatui/Crossterm. Không rewrite, không thêm mmap/piece-tree hoặc
LSP/plugin trong đợt giảm rủi ro. ViewState và ContentSnapshot tạo đường biên để
thay storage sau này mà không bắt TUI nhận Ropey trực tiếp; hiện tại chưa phải
backend-pluggable hoàn chỉnh.

Open dùng BufReader 64 KiB vào Ropey builder, không dựng Vec/String toàn file.
Save hash/encode theo chunks và giữ commit boundary. Windows copy file để giữ
ADS rồi viết lại main stream của temp; tốn thêm disk I/O nền để bảo toàn metadata.
DACL được bảo vệ để giữ effective access, có thể thay đổi inheritance semantics.
Nếu không giữ được metadata được hỗ trợ thì từ chối trước commit.

History tính inserted+deleted bytes và 4096 bytes/transaction, không đo chính xác
allocator/RSS. Một thay đổi lớn vẫn Undo được nhờ ngoại lệ transaction gần nhất.
Cache layout mỗi view tối đa 64 dòng/16 MiB text, ngoại lệ một dòng active quá lớn;
cache inactive toàn ứng dụng có target 32 MiB layout và 32 MiB review. Review nhận
input <=16 MiB, output <=100000 dòng/16 MiB charge. Source không bị cắt.

TUI search/review dùng chung hai CPU permits. Review dùng rendezvous channel để
kết quả tab ẩn không tích lũy; bỏ receiver cho worker tự giải phóng output. I/O,
recovery và Agent có giới hạn riêng; không diễn giải thành chỉ hai thread/process.
Full text/parser DOM vẫn có thể được cấp phát trên worker trong budget input.

Recovery một worker/queue một request, checkpoint khoảng mỗi giây khi có đổi.
Store plaintext owner-only, tối đa 64 MiB/draft, 256 MiB/store và 128 record/session
scan; không xóa draft cũ để tạo chỗ. Quota lỗi của một tab không chặn tab khác.
Crash lúc tạo checkpoint chỉ bảo đảm bản commit trước đó. Thông tin source là
provenance, tuyệt đối không dùng làm save destination. AI áp dụng secret/workspace
policy lên provenance cả sau crash lặp lại và Save As không đổi revision.

## Kiểm chứng

**127 Rust tests pass** (core 26, I/O 22, service 3+6, syntax 5, TUI 60,
CLI 4, file workflow 1). Fmt, strict native/Linux x64/macOS ARM64 cross-Clippy,
debug/release workspace builds và dependency audit đều pass. Các probe native
smoke/workspace/tabs/dialog/mouse/search/Agent sessions/Agent live/MCP/recovery
EN-VI/history đều pass trên binary release cuối.

| Workload cuối đợt | Số mẫu | Max ms |
| --- | ---: | ---: |
| Editor/review/dialog/tree | 196 | 36.155 |
| Tab/overflow manager | 26 | 44.172 |
| Dialog và mouse | 15 | 33.523 |
| Search/replace UI feedback | 13 | 70.618 |
| UI trong lúc MCP analyze | 28 | 14.348 |
| Navigation có trace, 12 phiên mới | 52 | 22.873 |
| Navigation không trace, 12 phiên mới | 52 | 18.905 |
| Sau 2401 edit transaction | 5 | 12.242 |

**387/387 phản hồi mới dưới 100 ms**, max 70.618 ms. Nearest-rank p95/p99 của
corpus hỗn hợp lần lượt 31.525/34.637 ms; chỉ mô tả mẫu này, không phải ước lượng
tail của mọi input. Navigation traced không mất sample. Transport ConPTY và
screen parsing được tính; physical paint không được đo. Các file JSON/CSV và
binary SHA-256 nằm trong [validation manifest](diagnostics/risk-after/validation.json).

Recovery EN/VI mỗi ngôn ngữ trải qua hai process kill sau checkpoint/handoff,
giữ exact BOM/NFD/ZWJ/mixed newline; external original không bị ghi đè; Save As
và clean exit retire draft đúng. Một lượt harness ban đầu gửi Quit sau disk commit
nhưng trước UI save receipt; probe đã sửa để chờ receipt. Giữ log thất bại riêng;
không sửa source để bỏ qua trạng thái đang save.

History stress 10 MiB: 2400 phím bulk hoàn tất trong 524.933 ms (throughput,
không phải một phản hồi), rồi 5 phản hồi tương tác max 12.242 ms. Observer yêu cầu
sample RSS mỗi 5 ms, thu 41 mẫu, peak quan sát 53219328 bytes (~50.75 MiB).
Timer/observer có thể bỏ qua spike; không gọi đây là hard peak/RSS bound.
Smoke idle 2.005 giây: 0 CPU seconds và 0 output bytes, RSS 7524352 bytes.

Microbenchmark hiện có chạy 21 sample/operation/size, không qua TUI. 10 MiB warm
open p50/p95 107.817/126.219 ms, save 100.931/137.572 ms, random edit+Undo
0.012/0.018 ms, search-absent 15.575/18.331 ms. Open chưa gồm TUI prewarm.
Repeated saves pass với 1 KiB/1 MiB/10 MiB. I/O/parse completion không được gộp
vào số feedback: search/replace-all và MCP analysis có thể vượt 100 ms trên worker.
Raw benchmark nằm tại diagnostics/risk-after/editor-benchmark.csv.

Kiểm thử mới tập trung vào dữ liệu và các đường hỏng thực tế:

- Same bytes/mtime nhưng object identity khác vẫn xung đột; hardlink không bị
  âm thầm tách; Windows ADS + owner-only ACL giữ nguyên khi save.
- UTF-8 BOM, CRLF qua biên chunk 64 KiB, NFD/ZWJ, mixed newlines giữ đúng bytes.
- Kill/restart sau checkpoint; không claim editor còn sống; checksum hỏng được
  giữ lại; quick quit trước khi thấy draft không xóa draft; original đổi bên ngoài
  vẫn nguyên; crash lần hai giữ provenance; Save As cập nhật metadata checkpoint.
- Snapshot worker không history, Undo quota giữ savepoint, ViewState từ chối
  revision cũ; cache nhiều dòng/tab width/Undo/Redo bằng kết quả layout mới.
- Tô màu sau horizontal clip giữ string context; context vượt budget plain;
  monochrome reset cả source lẫn dialog; review bị bỏ không giữ hết permits.
- Recovered .env/outside-workspace không lách được AI read/list policy.

Baseline chẩn đoán trước thay đổi cache mới: 32 phản hồi, max 22.587 ms;
probe ad-hoc dòng dài thứ hai có End 77.021 ms. Đây là mẫu đo mới trước tối ưu,
không thay thế baseline 916.326 ms và không đủ để suy ra hệ số tăng tốc.
Raw files được giữ trong docs/diagnostics/risk-baseline/.

## Chẩn đoán bổ sung: thoát dialog ~500 ms

Probe dialog phát hiện exit 509.377 ms; các lần lặp với trace tái hiện
526.226/520.871/532.115 ms. Trace khoanh vùng raw terminal restore ~512 ms,
trong khi recovery finish ~1.3 ms và input handler ~0.01 ms. Raw PTY chứa
ESC[6n không được trả lời: pyte.Screen.write_process_input mặc định là no-op.

Harness đã nối callback trả lời terminal status/cursor position về PTY, như
terminal tương tác. Cùng probe sau đó pass 15 thao tác, max 29.779 ms;
raw restore còn 0.159 ms, terminal restore 1.224 ms. Giữ JSON thất bại và CSV
trước/sau trong risk-after. Đây là chẩn đoán lỗi giao thức của harness, không
phải bằng chứng nguyên nhân cho cold End 916.326 ms lịch sử. Các số đo trước/sau
sửa harness được phân biệt, không xóa hoặc sửa raw failure.

## Rủi ro còn mở và cổng tiếp theo

1. **P0 còn mở:** nguyên nhân cold End 916.326 ms. Trace hiện có giúp thu evidence
   lần sau; các lần pass không xóa failure cũ. Cần trace scheduler/terminal khi
   tái hiện và đo p95/p99 với nhiều cold sessions trên máy người dùng.
2. Chạy native filesystem/recovery tests trên Linux/macOS và Windows ARM64;
   POSIX ownership/xattr/ACL, junction/symlink privileges và filesystem khác.
3. Thử disk-full thật, power cut có kiểm soát và antivirus/network filesystem;
   không đánh đồng injected errors/process kill với những thử nghiệm đó.
4. Theo dõi RSS/allocator khi nhiều tab/DOM và transaction lớn. History/cache
   charge không phải bằng chứng không thể OOM; mở >10 MiB vẫn cần xác nhận.
5. Shared parser-neutral models, document store/editable views và large-file
   backend là milestone riêng có semantic tests; không đưa vào hot path sớm.

Không thể tuyên bố loại bỏ final-check race bằng một rename portable. Chính sách
hiện tại phát hiện và từ chối các trường hợp xác định được, giữ thông báo đúng
trước/sau commit, và yêu cầu Save As cho trường hợp bảo toàn tên/metadata không đủ.

## Đính chính checkpoint nghiên cứu

Báo cáo 06 đã nói workspace dependency guinea-service thiếu version. Đối chiếu
Cargo.toml ở baseline cho thấy version = 0.1.0 đã có. Đây là lỗi báo cáo, không
phải thay đổi implementation lần này. Giữ nghiên cứu gốc để có lịch sử và ghi rõ
đính chính ở checkpoint mới/Notion; không lấy lỗi này làm lý do claim sửa build.

## Cách tái chạy

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
python scripts/pty_recovery.py
python scripts/pty_navigation.py --trace
python scripts/pty_risk_stress.py
```

GUINEANOTE_TRACE nhận đường dẫn chưa tồn tại; ứng dụng từ chối ghi đè trace.
Queue 4096 sample dùng try_send. CSV có duration, since-start timestamp và
count dropped_samples lúc tắt. Không log nội dung, phím, path hay clipboard.
Trace tạo overhead; so sánh số đo traced/untraced trong đúng phạm vi.
