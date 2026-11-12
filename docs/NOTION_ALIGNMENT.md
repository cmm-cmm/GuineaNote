# Đối chiếu quyết định Notion — 10/11/2026

## Cập nhật 12/11/2026

- Notion: dự án chỉ còn **một** dòng "Guinea Note" trong Projects; workstream độ
  trễ được chuyển thành trang con, nội dung giữ nguyên. Docs liên kết: tổng quan,
  bảng phím v1, thao tác soạn thảo (M6), báo cáo dòng dài chưa lập chỉ mục.
- Chính sách xung đột phím đã chốt: giữ phím hiện có, thao tác mới theo Notepad++
  khi terminal truyền được, mỗi thao tác có phím chính portable và luôn chạy được
  qua F2 ([bảng phím](KEYBOARD_SHORTCUTS.md)).
- M6 (soạn thảo cốt lõi) và phần tùy chọn tìm kiếm Match case/Whole word, OSC 52 của
  M8 đã triển khai; M7 vẫn mở. Chi tiết commit: [trạng thái](IMPLEMENTATION_STATUS.md).

Nguồn chuẩn hiện hành:

- [Project Guinea Note](https://app.notion.com/p/41127da1dc088160b076e368c4858aad), cập nhật 06:17 UTC.
- [Kế hoạch G0–G4](https://app.notion.com/p/41427da1dc0881fbbab2fe7da87aa2fb), cập nhật 06:17 UTC.
- [Đặc tả latency](https://app.notion.com/p/41427da1dc0881309c02cf504b7c59dd).

## Quyết định được áp dụng

Tên sản phẩm là **Guinea Note**, executable đích là **`guinea`**. Repository
`cmm-cmm/GuineaNote`, Cargo package `guineanote`, crate IDs, config/preferences,
recovery paths, biến môi trường GUINEANOTE_* và Agent/MCP IPC giữ nguyên.
Không suy diễn package name còn khả dụng và không publish. Đổi executable là
hạng mục riêng với [chính sách migration](CLI_MIGRATION.md); không trộn với sửa save/text engine.
Các nghiên cứu và raw benchmark cũ giữ nguyên tên, hash và số liệu lịch sử.

Terminal/TUI vẫn là sản phẩm chính: Rust native, Ropey, Ratatui/Crossterm,
editor state một owner. Không triển khai GUI, LSP, plugin ABI hoặc paged editor
chỉ vì định hướng mới. File safety, Unicode, history và quyền Agent ưu tiên trước.

Mục tiêu tương tác: p50 ≤10 ms, p95 ≤20 ms, p99 ≤50 ms theo category/corpus/phase.
Mọi phản hồi >50 ms phải điều tra theo yêu cầu mới nhất, thay ngưỡng 100 ms cũ.
Cold End 916.326 ms còn mở; không cam kết SLA toàn cục từ một tập đo.
Các checks chạy local; GitHub Actions chưa kích hoạt theo chỉ đạo người dùng.

## Thứ tự thực hiện và trạng thái

Điều chỉnh trực tiếp mới nhất của người dùng: **không cần kiểm chứng Ubuntu/SSH
trong đợt tiếp theo; ưu tiên kiểm chứng độ trễ**. Các mục native/SSH trong roadmap
bên dưới giữ như khoảng trống evidence, không chặn đợt latency hoặc buộc chuẩn bị
môi trường remote. Xem [kế hoạch latency hiện hành](performance/INTERACTION_LATENCY_TARGETS.md).

| Gate | Đối chiếu hiện tại | Việc còn lại |
| --- | --- | --- |
| G0 | HEAD nền f2ac421; trace input-to-flush, corpus phân nhóm và raw evidence mới | Cold-session/tail mở rộng, root cause outlier lịch sử |
| G1 | Safety/recovery hiện có được giữ; native Windows probes | Linux/macOS/ARM native, SSH/tmux thật, power-loss và filesystem matrix |
| G2 | Project picker/search, palette gợi ý, goto, focus mode, mono/ANSI16 | Registry dispatcher/disabled reasons thống nhất hoàn toàn; OSC52 opt-in; lexical context liên dòng |
| G3 | ContentSnapshot/ViewState và quota đã có | DocumentId/ViewId store, editable split và paged prototype chỉ sau use case/benchmark |
| G4 | Chuẩn hóa executable guinea là task riêng | Native release matrix, checksums/SBOM/install/rollback; không tự bật CI/publish |

Không gọi cross-Clippy là chứng nhận runtime. Không đánh dấu toàn bộ gate Done
khi một phần evidence chưa có. Số 127 tests và 387 phản hồi trên Notion là checkpoint
lịch sử; số mới được ghi riêng trong status/baseline cùng hash và phạm vi.

Đính chính nghiên cứu: dependency workspace `guinea-service` đã có version 0.1.0
ở baseline; nhận xét thiếu version trong snapshot nghiên cứu 06 không đúng.

## Tiếp tục theo yêu cầu người dùng — 11/11/2026

Đã đọc trực tiếp project chính, kế hoạch G0–G4, workstream và đặc tả latency
trên Notion. Các trang vẫn ghi source fe46835; HEAD local thực tế là 3cca7bf,
chứa checkpoint ASCII mới hơn. Manifest của checkpoint khớp đủ 166 artifact.
Yêu cầu tiếp tục hiện tại thay trạng thái tạm dừng trước đó; tài liệu pause và
raw evidence cũ được giữ làm lịch sử.

Hạng mục tiếp tục: profiling observer search và phân biệt feedback/completion,
theo phần còn mở của checkpoint ASCII. Đã bổ sung legacy/corrected comparison,
trace/profiling on/off trên cùng binary, cùng kiểm tra Unicode/file bytes/Undo,
readonly và cleanup. [Báo cáo](performance/SEARCH_OBSERVER_INVESTIGATION.md)
ghi kết quả và hash của đợt này. Thay đổi còn ở working tree trên nền 3cca7bf;
không suy ra trạng thái toàn bộ G0–G4 đã Done.

Đã thêm checkpoint 11/11 trực tiếp vào project chính và
[workstream latency](https://app.notion.com/p/41427da1dc088110a64bc768bbf90ed0),
ghi rõ kết quả kiểm chứng, giới hạn và trạng thái chưa commit/push.

## Yêu cầu tiếp theo: TUI thao tác hoàn toàn bằng phím

Người dùng yêu cầu hợp nhất Find/Replace và rà soát phím cho toàn bộ giao diện,
đặc biệt quản lý/chuyển tab. Đã triển khai theo yêu cầu trực tiếp này; xem
[bảng phím](KEYBOARD_SHORTCUTS.md) và [kiểm chứng](KEYBOARD_AUDIT.md).
146 Rust tests và 12 native probes pass, gồm mouse=false và bảo vệ dirty/AI.
Không suy ra toàn bộ G2 hoặc gate phát hành đã hoàn thành.

Điều chỉnh phím trực tiếp tiếp theo: F6 thẻ trước, F7 thẻ sau, F8 Review,
F10 Settings; Alt+M mục lục để tránh trùng F10. Đã cập nhật cả dispatcher,
toolbar/F1/F2/VI–EN và native probes. Kiểm chứng mới: 147 Rust tests và 13
native probes pass; [evidence riêng](diagnostics/function-keys/summary.json).

## Chốt checkpoint theo yêu cầu commit/push

Commit bao chứa tài liệu này gom contributor guide, correction của observer,
bảng Find/Replace hợp nhất, điều hướng bàn phím và mapping cuối cùng. Các raw
report giữ nguyên trạng thái lúc đo trước commit. Source/binary hiện hành đã
đối chiếu khớp hash với checkpoint 147 Rust tests và 13 native probes; đủ 50
artifact của ba manifest mới khớp checksum. Commit hash cuối cùng được ghi trên
project và workstream Notion sau khi đối chiếu remote dev. Các gate còn mở và
giới hạn Windows/ConPTY vẫn được giữ, không đổi thành Done từ kết quả local.
