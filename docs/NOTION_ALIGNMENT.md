# Đối chiếu quyết định Notion — 10/11/2026

Nguồn chuẩn hiện hành:

- [Project Guinea Note](https://app.notion.com/p/41127da1dc088160b076e368c4858aad), cập nhật 06:17 UTC.
- [Kế hoạch G0–G4](https://app.notion.com/p/41427da1dc0881fbbab2fe7da87aa2fb), cập nhật 06:17 UTC.
- [Đặc tả latency](https://app.notion.com/p/41427da1dc0881309c02cf504b7c59dd).

## Quyết định được áp dụng

Tên sản phẩm là **Guinea Note**, executable đích là **`guinea`**. Repository
`cmm-cmm/GuineaNote`, Cargo package `guineanote`, crate IDs, config/preferences,
recovery paths, biến môi trường GUINEANOTE_* và Agent/MCP IPC giữ nguyên.
Không suy diễn package name còn khả dụng và không publish. Đổi executable là
hạng mục riêng với kiểm thử migration; không trộn với sửa save/text engine.
Các nghiên cứu và raw benchmark cũ giữ nguyên tên, hash và số liệu lịch sử.

Terminal/TUI vẫn là sản phẩm chính: Rust native, Ropey, Ratatui/Crossterm,
editor state một owner. Không triển khai GUI, LSP, plugin ABI hoặc paged editor
chỉ vì định hướng mới. File safety, Unicode, history và quyền Agent ưu tiên trước.

Mục tiêu tương tác: p50 ≤10 ms, p95 ≤20 ms, p99 ≤50 ms theo category/corpus/phase.
100 ms là ngưỡng điều tra; không phải SLA toàn cục. Cold End 916.326 ms còn mở.
Các checks chạy local; GitHub Actions chưa kích hoạt theo chỉ đạo người dùng.

## Thứ tự thực hiện và trạng thái

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
