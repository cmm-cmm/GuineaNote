# Cải thiện phản hồi và ngưỡng điều tra 50 ms

## Quyết định hiện hành

Mọi phản hồi tương tác **>50 ms phải điều tra**, dù percentile nhóm đạt. Giữ
p50 ≤10 ms, p95 ≤20 ms, p99 ≤50 ms; cold/warm, corpus và context báo riêng.
Kiểm chứng local Windows/ConPTY; Ubuntu/SSH không là gate của đợt này.
GitHub Actions chưa kích hoạt. Báo cáo chứa kết quả đo, không phải SLA toàn cục.

## Thay đổi đã triển khai

- Display cache dùng logical head để giữ/tái dùng khoảng trống ASCII đầu dòng,
  tránh dịch suffix nhiều MiB ở prefix Backspace/Undo/Redo lặp. Chỉ áp dụng
  với index đầy đủ, không tab và điều kiện ASCII an toàn; Unicode/tab/mid-line
  giữ fallback. Ropey là source of truth. Không thêm scratch buffer lớn; charge
  tính cả prefix còn giữ, không phải hard RSS/capacity bound.
- Sau replacement explicit, refresh kết quả được chạy ngay thay vì chờ debounce
  dành cho gõ query. Giữ worker admission, revision/readonly/lease và one-step Undo.
- Menu chính có một hàng phân cách với toolbar quản lý tab; hit regions tách biệt
  trên VI/EN, welcome/editor, 30×9–148×42. [Ảnh native](../diagnostics/latency50/menu-tabs-separated.png).
- Harness đánh dấu mọi mẫu >50 ms trước cả kiểm tra đủ n/percentile. Trace có
  agent_active tại receipt; canonical executable và process-name filter được sửa.

## So sánh TUI trước/sau

Cùng corpus, release, terminal 100×28 và 100 vòng thao tác. Hàng phân cách mới
giảm viewport editor một hàng: đây là so sánh hai phiên bản app, không phải
viewport-equivalent isolation của riêng cache. Microbenchmark core bên dưới
tách UI; binary/hash và mọi raw samples có trong [manifest](../diagnostics/latency50/manifest.json).

| Unicode long line 10 MiB | Trước p50/p95/p99/max ms | Sau p50/p95/p99/max ms | n mỗi lượt |
| --- | --- | --- | ---: |
| typing | 3.102 / 5.554 / 5.729 / 6.223 | 1.656 / 3.317 / 3.451 / 3.632 | 124 / 124 |
| delete | 3.887 / 20.03 / 20.455 / 20.674 | 1.671 / 3.238 / 3.337 / 3.351 | 124 / 124 |
| caret | 1.362 / 2.85 / 3.103 / 3.144 | 1.266 / 2.932 / 3.072 / 3.15 | 199 / 199 |
| history | 5.846 / 20.07 / 22.699 / 22.699 | 3.016 / 3.293 / 3.417 / 3.417 | 49 / 49 |

Trước: delete/history p95 20.03/20.07 ms không đạt mục tiêu 20 ms; **không có**
mẫu tương tác >50 ms trong corpus này. Sau: 2050 input-to-flush samples, 2016
warm samples/28 nhóm; các nhóm warm đủ n đạt, all-phase max 10.558 ms,
zero dropped. Raw [trước](../diagnostics/latency50/before/report.json),
[sau](../diagnostics/latency50/final-trace/report.json),
[trace off](../diagnostics/latency50/final-no-trace/report.json) cùng binary cuối.
PTY max trace-on/off 27.944/26.523 ms, không mẫu >50 ms. Không dùng PTY làm internal timing.

## Core microbenchmark

1000 vòng insert/backspace/undo/redo trên prefix dòng 10 MiB, có prepare_view;
không gồm terminal/render. Chỉ số ms:

| Core operation | Trước p95/p99/max | Sau p95/p99/max |
| --- | --- | --- |
| insert | 2.23 / 2.483 / 2.834 | 0.041 / 0.1 / 1.131 |
| delete | 15.466 / 18.229 / 18.94 | 0.041 / 0.097 / 0.118 |
| undo | 2.195 / 2.468 / 2.659 | 0.037 / 0.041 / 0.089 |
| redo | 14.579 / 17.915 / 19.133 | 0.037 / 0.041 / 0.119 |

Giữ cả lượt before đầu và isolation. Hash binary microbenchmark before chưa
được thu; không coi bảng này là certification. String reference thay API insert
giữa hai lượt, nên **không so reference như benchmark kiểm soát** hoặc suy ra
lỗi stdlib/hệ số nhanh hơn chung. Cache-prefix tránh dịch suffix theo cấu trúc code.
Đã đối chiếu Rust 1.98.1 local alloc/string.rs và [source chính thức](https://doc.rust-lang.org/src/alloc/string.rs.html)
về replace_range/Vec splice; chi phí thực tế phụ thuộc máy và compiler.

## Độ phủ mở rộng — binary cache-only

Đo trước thay đổi refresh search cuối; không gộp percentile với binary cuối.

- [Extended](../diagnostics/latency50/cache-only-expanded/report.json): 9250 samples, max 10.786 ms, không mẫu >50 ms; typing/delete warm 1024 mẫu mỗi nhóm/corpus.
- [Cold navigation](../diagnostics/latency50/cold-navigation/report.json): 106 phiên mới, 100 first-End Unicode cold samples.
  First-End p50/p95/p99/max 1.175 / 1.789 / 2.202 / 2.429 ms; toàn corpus app max 13.401 ms, PTY max 25.031 ms.
  Cold nghĩa là process/category đầu tiên, line đã được open-worker prewarm; không phải cold disk.
- [Agent](../diagnostics/latency50/agent-contention/report.json): 10 phiên, 280 samples tại receipt có Agent active, max 3.861 ms; zero dropped.
  Caret/typing warm mỗi nhóm n=90 đạt; nhóm n<20 giữ insufficient_samples.
  Active lease/state không chứng minh CPU execution đồng thời tại từng event.

## Điều tra mẫu >50 ms và đính chính

1. Search regression ban đầu ghi **71.135 ms**, tái lập với trace **73.491 ms**
   khi chờ No matches. Tất cả app input-to-flush trong lượt tái lập ≤5.386 ms.
   Source applied() khởi động lại typing debounce dù replacement explicit đã xong.
   Harness cũ gộp result completion vào feedback; giữ nguyên raw failure trong
   [investigations/search-original](../diagnostics/latency50/investigations/search-original/report.json)
   và [traced](../diagnostics/latency50/investigations/search-traced/report.json).
   Bản mới báo feedback và completion riêng; không xóa/relabel raw cũ.
   Sau khi bỏ debounce thừa, small replacement + refresh completion vẫn 55.618 ms trong lượt này,
   không phải chứng minh background completion ≤50 ms. Phản hồi PTY max 49.7 ms;
   input-to-flush max 6.01 ms. [Raw cuối](../diagnostics/latency50/final-search/report.json).
2. Collector Agent ban đầu dừng vì CSV không có agent_active: pty_agent.py ghép
   executable guineanote + SUFFIX, chạy artifact cũ. Giữ log lỗi/schema cũ tại
   investigations/stale-agent-binary. Đã sửa guinea + SUFFIX và chạy lại 10 phiên;
   Agent của checkpoint CLI migration cũ chưa chứng minh guinea.exe. Các số liệu
   cũ không bị đổi; có đính chính ở tài liệu migration.
3. Cold End lịch sử **916.326 ms vẫn chưa rõ nguyên nhân**, không tái lập trong
   100 first-End Unicode sessions mới. Chưa có trace lịch sử đủ để kết luận
   OS scheduling/adapter/harness hay ứng dụng. Không tự đóng rủi ro.

## Kiểm chứng và việc còn mở

136 Rust tests, fmt, strict native Clippy và workspace release build pass.
Native Windows probes byte fidelity/BOM/EOL/Undo/readonly/Agent/recovery, tab/mouse
và terminal matrix 6/6 pass. Lượt regression trước sửa search có một failure được
giữ; [bản cuối](../diagnostics/latency50/regression-final/results.json) pass toàn bộ.
Idle probe: 2.0 s, 0 CPU seconds, 0 output bytes, RSS 7581696 bytes (small buffer;
không phải peak RAM).
Không dùng các phép đo này thay adapter queue/decode profiler, physical paint,
stress resize dồn dập hoặc chứng nhận mọi workload. Khả năng SSH không được
kiểm chứng theo scope hiện tại; không bật Actions hoặc publish package.
