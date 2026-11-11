# Điều tra observer search — 11/11/2026

Đã xác định một lỗi định nghĩa feedback trong harness replace-all: predicate
cũ bỏ qua frame `Searching`, tiếp tục đợi `No matches` và tính cả refresh kết quả
vào độ trễ phản hồi ban đầu. Predicate mới nhận `Searching` như phản hồi hợp lệ;
completion vẫn chờ riêng. Đây là sửa phép đo, không phải tối ưu tốc độ editor.

## Nền và phạm vi

Người dùng yêu cầu đọc Notion và tiếp tục công việc. Đã đọc project chính, kế
hoạch G0–G4, workstream và đặc tả latency; Notion còn ghi fe46835, HEAD local là
`3cca7bfaf7f494f7f7c355b0803d172d4942a827`. Toàn bộ 166 artifact trong manifest
checkpoint ASCII khớp checksum. Tài liệu tạm dừng giữ nguyên làm lịch sử.

Đợt này thay đổi harness, diagnostic evidence và tài liệu trong working tree.
Rust editor/core/Unicode/save/history/recovery không thay đổi. Native Windows
11 x64/MSVC, Rust 1.98.1, Python 3.15.2, ConPTY; dependency versions ghi trong
[environment.json](../diagnostics/search-observer/environment.json).

Binary SHA256 của toàn bộ matrix:
`4b0261d9f775d1965d08ee6a3931bbff1bfe6b95e31a79f7460612fb56444c51`.
Source SHA256 theo collector hiện có:
`e4e9e0c27ccdd0c30b7b5e46cd22dd4b504f70906947466ecd663ba9adfe70b6`.
Hash mới khác checkpoint lịch sử; chỉ đối chiếu các điều kiện của matrix mới
trên cùng binary/source/harness. Không gộp số liệu hai checkpoint.

## Bằng chứng nguyên nhân tái lập

Trong [lượt legacy đại diện](../diagnostics/search-observer/representative-legacy/report.json),
observer thấy `Searching` ở **10.042 ms**, nhưng predicate chỉ kết thúc sau khi
thấy `No matches` ở **58.408 ms**; sample tổng là **58.431 ms**. Trong đó:

- select wait: 51.981 ms;
- receive: 0.125 ms; decode + pyte feed: 0.871 ms;
- append: 0.033 ms; phần ngoài các đoạn được đo: 5.389 ms.

Frame `Searching` xuất hiện khi replacement nhỏ đã được áp dụng và kết quả đang
refresh. Worker có thể hoàn thành trước frame `Replacing`; vì vậy chỉ chấp nhận
Replacing/No matches làm bỏ sót feedback sớm. Predicate được kiểm tra false
trước click để tránh nhận trạng thái cũ.

[Lượt corrected cùng số thứ tự](../diagnostics/search-observer/representative-corrected/report.json)
ghi nhận feedback 7.481 ms, thấy Searching ở 7.460 ms. Đây là phiên riêng cùng
binary, không phải phép trừ hai timestamp của cùng một event. Mỗi lượt vẫn chờ
No matches, kiểm tra Undo và file bytes trước khi kết thúc.

## Matrix đối chiếu

20 vòng × 8 điều kiện: legacy/corrected × trace on/off × observer profiling
on/off. Xoay thứ tự điều kiện theo vòng, chạy tuần tự sau build/checks. Mỗi lượt
gồm ba phiên: Unicode nhỏ với BOM/CRLF/NFD/ZWJ, readonly, và search/replace-all
10 MiB. Tổng **160 lượt, 480 phiên native, 2.080 PTY samples**.

Small replace-all PTY, đơn vị ms; **n=20 cho mỗi ô trước/sau**:

| Điều kiện | Legacy p95 / max | Legacy >50 ms | Corrected p95 / max | Corrected >50 ms |
| --- | ---: | ---: | ---: | ---: |
| Trace off, plain | 57.992 / 58.296 | 9 | 12.256 / 12.264 | 0 |
| Trace off, profiled | 57.482 / 57.498 | 6 | 12.459 / 12.555 | 0 |
| Trace on, plain | 57.987 / 58.238 | 7 | 12.606 / 13.296 | 0 |
| Trace on, profiled | 56.800 / 58.431 | 3 | 12.470 / 12.477 | 0 |

Legacy giữ đủ **25 mẫu >50 ms**. Corrected có **1.040 PTY samples**, max
**32.167 ms**, không mẫu >50 ms trong toàn workflow. Các percentile được tính
từ raw và tách theo điều kiện/category; bảng chỉ trình bày small replace-all.

Trace-on của cả hai định nghĩa có **5.920 input-to-flush samples**, max
**8.567 ms**, không mẫu >50 ms và zero dropped. 96 nhóm
condition/session/operation/phase đạt các mục tiêu theo phép tính collector;
các nhóm cold chỉ có 20 mẫu. Không dùng percentile hỗn hợp hoặc coi n=20 là
chứng nhận tail dài hạn.

Completion được giữ riêng. Small replace-all corrected p95 theo bốn điều kiện
là 45.424–46.208 ms; legacy p95 là 59.736–61.176 ms. Việc chuyển thời điểm
observer bắt đầu chờ completion cũng thay đổi hành vi đọc/wait, nên chênh lệch
này không chứng minh worker chạy nhanh hơn. Completion 10 MiB không chịu mục
tiêu feedback 50 ms. Xem đầy đủ [summary](../diagnostics/search-observer/summary.json).

## Kiểm chứng và tái chạy

140 Rust tests, fmt, strict native Clippy, release workspace build và ba Python
reader tests pass. Reader tests bao gồm UTF-8 bị chia giữa các chunk, output
rỗng và transport bị đóng. Native smoke pass: idle 2.008 s, CPU 0 s, output
0 bytes; cleanup bình thường. Các lượt matrix giữ Unicode/BOM/CRLF, readonly,
explicit save, Undo và source-byte assertions của workflow search hiện có.

```powershell
cargo build --workspace --release
python -X utf8 -m unittest discover -s scripts -p test_pty_observer.py -v
python -X utf8 scripts/pty_search.py --trace --profile-observer
python -X utf8 scripts/pty_search_matrix.py --runs 20 --record-only
```

`--record-only` là chế độ điều tra, không chuyển outlier thành pass. Các lỗi
correctness, trace thiếu footer/missing hoặc dropped vẫn thất bại. Bỏ flag này
để matrix fail khi corrected có phản hồi >50 ms. Cài PTY development dependencies
và Pillow theo [test plan](../TEST_PLAN.md); không build/probe khác đồng thời.

## Evidence và giới hạn còn mở

- [Raw matrix ZIP](../diagnostics/search-observer/matrix-raw.zip) chứa đủ console,
  report, CSV và terminal logs của 160 lượt; [entry checksums](../diagnostics/search-observer/matrix-checksums.json)
  đối chiếu từng file sau giải nén. Raw pilot trước sửa giữ riêng trong
  [pilot ZIP](../diagnostics/search-observer/pilot-raw.zip).
- [Harness snapshot](../diagnostics/search-observer/harness-snapshot.zip) và
  [fingerprints](../diagnostics/search-observer/harness-sha256.json) lưu đúng
  scripts dùng trong matrix. [Manifest](../diagnostics/search-observer/manifest.json)
  kiểm tra bytes của bundle; logs kiểm chứng nằm cùng thư mục.
- Select wait chưa tách được adapter queue, transport và OS scheduling;
  `observer_unattributed_ms` không phải thời gian editor. Các timer có overhead;
  bốn điều kiện on/off được báo riêng. Process CPU trên Windows có độ phân giải
  thô; timestamp ghi lúc observer thấy trạng thái, không phải lúc pixel được vẽ.
- Đã tái lập và chứng minh confound của predicate cũ. Mẫu lịch sử search
  **55.417 ms** không có telemetry trạng thái tương đương; chưa gán chắc chắn
  toàn bộ độ trễ của chính mẫu đó. Cold End **916.326 ms** vẫn chưa rõ nguyên nhân.
- Soak RAM nhiều tab, queue/decode profiling chi tiết và kiểm chứng native OS
  khác còn mở. GitHub Actions/publication và các gate G0–G4 chưa được nghiệm thu.
