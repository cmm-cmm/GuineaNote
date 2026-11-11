# Baseline độ trễ tương tác — 10/11/2026

Đợt tiếp tục 11/11/2026: [điều tra observer search](SEARCH_OBSERVER_INVESTIGATION.md)
đối chiếu hai định nghĩa feedback trên cùng binary mới build. Đây là correction
của harness; không thay baseline input-to-flush hoặc số liệu lịch sử bên dưới.

Checkpoint executable mới: [kiểm chứng guinea](CLI_MIGRATION_VALIDATION.md),
2016 warm samples / 28 nhóm, all-phase max 22.349 ms. Phần dưới giữ baseline
trước migration (checkpoint TUI af420c5), không thay số liệu lịch sử.

Native Windows x64/MSVC release, ConPTY + pyte; viewport 100×28, resize 80×24.
Đây là baseline của phiên bản có thay đổi chưa commit trên nền `f2ac4214dc8ac19692c3ae4bebac6b86ff8702a3`.
Commit bao chứa tài liệu này xác định checkpoint; source/binary SHA256 có trong
[manifest](../diagnostics/terminal-workflow/manifest.json) và raw report.

- Source SHA256: `a4d3063de5c550a42cbafa5565cdb3078543d77763198eb595d78e0329bc8cdf`.
- Binary SHA256: `c22358ff4c6254eebcc5f28695f8b9c9a4a673f3e365f45110fe948bef365cd5`.
- [Trace/raw CSV](../diagnostics/terminal-workflow/trace/report.json).
- [Cùng corpus trace off](../diagnostics/terminal-workflow/no-trace/report.json).

## Application input-to-flush — warm

Theo [phương pháp](BENCHMARK_METHODOLOGY.md), nearest-rank; số liệu ms.
Không tổng hợp percentile giữa corpus hoặc loại thao tác.

| Corpus | Operation | n | p50 | p95 | p99 | max | >100 ms | Kết quả |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| small | caret | 250 | 1.305 | 3.83 | 4.163 | 4.329 | 0 | pass |
| small | delete | 124 | 1.348 | 2.984 | 3.157 | 3.247 | 0 | pass |
| small | dialog_input | 29 | 2.445 | 4.263 | 5.756 | 5.756 | 0 | pass |
| small | dialogs | 29 | 2.511 | 4.707 | 5.724 | 5.724 | 0 | pass |
| small | history | 49 | 2.945 | 3.177 | 3.369 | 3.369 | 0 | pass |
| small | mouse | 49 | 2.893 | 3.184 | 3.266 | 3.266 | 0 | pass |
| small | project_open | 24 | 4.648 | 4.9 | 4.923 | 4.923 | 0 | pass |
| small | project_query | 49 | 3.786 | 4.566 | 5.915 | 5.915 | 0 | pass |
| small | resize | 74 | 4.358 | 4.777 | 5.078 | 5.078 | 0 | pass |
| small | search_query | 99 | 3.304 | 4.598 | 4.744 | 4.744 | 0 | pass |
| small | search_ui | 49 | 4.617 | 5.139 | 5.219 | 5.219 | 0 | pass |
| small | tabs | 60 | 2.863 | 3.204 | 3.339 | 3.339 | 0 | pass |
| small | typing | 124 | 1.382 | 3.054 | 3.206 | 3.646 | 0 | pass |
| unicode-long-line | caret | 199 | 1.531 | 3.009 | 3.289 | 7.786 | 0 | pass |
| unicode-long-line | delete | 124 | 4.952 | 19.739 | 20.697 | 20.956 | 0 | pass |
| unicode-long-line | dialog_input | 29 | 4.021 | 4.314 | 5.767 | 5.767 | 0 | pass |
| unicode-long-line | dialogs | 29 | 4.238 | 4.784 | 4.795 | 4.795 | 0 | pass |
| unicode-long-line | history | 49 | 6.48 | 19.865 | 20.363 | 20.363 | 0 | pass |
| unicode-long-line | long_line_end | 24 | 2.909 | 3.029 | 3.165 | 3.165 | 0 | pass |
| unicode-long-line | long_line_home | 25 | 2.864 | 3.042 | 3.073 | 3.073 | 0 | pass |
| unicode-long-line | mouse | 49 | 2.4 | 2.65 | 2.891 | 2.891 | 0 | pass |
| unicode-long-line | project_open | 24 | 4.673 | 5.058 | 5.153 | 5.153 | 0 | pass |
| unicode-long-line | project_query | 49 | 3.392 | 4.318 | 4.417 | 4.417 | 0 | pass |
| unicode-long-line | resize | 74 | 4.073 | 4.68 | 4.816 | 4.816 | 0 | pass |
| unicode-long-line | search_query | 99 | 3.59 | 4.599 | 4.883 | 4.883 | 0 | pass |
| unicode-long-line | search_ui | 49 | 4.713 | 5.484 | 5.695 | 5.695 | 0 | pass |
| unicode-long-line | tabs | 60 | 2.841 | 3.21 | 3.261 | 3.261 | 0 | pass |
| unicode-long-line | typing | 124 | 3.673 | 5.799 | 5.9 | 6.137 | 0 | pass |

Không mất trace samples: 0. Các nhóm cold có n nhỏ và **không đủ**
để nghiệm thu p95/p99. Các nhóm warm có ít nhất 20 mẫu ở đây đạt mục tiêu;
p99 với n≈20–30 gần max, không phải chứng minh tail dài hạn. Agent contention
có probe riêng 28 PTY observations; chưa có đủ input-to-flush theo nhóm trong
corpus Agent để nghiệm thu mục tiêu mới.

## Trace overhead và phép đo ngoài ứng dụng

Cùng binary: True. Trace-off không cung cấp internal latency.
Dưới đây là PTY observer cho từng thao tác/corpus (ms); biến động phiên, scheduling
và transport vẫn ảnh hưởng, không suy ra hệ số overhead chỉ từ hai lượt.

| PTY operation | trace on p95/max | trace off p95/max |
| --- | ---: | ---: |
| caret:small | 6.984 / 7.445 | 4.155 / 7.62 |
| caret:unicode-long-line | 12.065 / 16.943 | 13.355 / 14.911 |
| delete:small | 7.321 / 8.286 | 5.123 / 5.889 |
| delete:unicode-long-line | 27.136 / 28.331 | 27.64 / 29.838 |
| dialog-close:small | 20.606 / 21.922 | 7.408 / 7.961 |
| dialog-close:unicode-long-line | 23.068 / 28.152 | 20.55 / 25.239 |
| dialogs:small | 21.494 / 21.677 | 5.64 / 5.884 |
| dialogs:unicode-long-line | 22.233 / 23.28 | 22.902 / 24.505 |
| history-type:small | 13.071 / 14.568 | 3.673 / 4.145 |
| history-type:unicode-long-line | 13.439 / 13.607 | 13.274 / 14.042 |
| long-end:small | 14.981 / 17.464 | 3.521 / 4.196 |
| long-end:unicode-long-line | 14.621 / 17.146 | 11.737 / 12.5 |
| long-home:small | 14.763 / 14.946 | 3.752 / 4.032 |
| long-home:unicode-long-line | 13.389 / 13.872 | 12.057 / 12.206 |
| mouse:small | 12.091 / 12.887 | 12.834 / 13.796 |
| mouse:unicode-long-line | 10.225 / 12.412 | 12.595 / 13.542 |
| project-close:small | 13.065 / 15.574 | 18.356 / 21.157 |
| project-close:unicode-long-line | 13.089 / 13.123 | 12.954 / 13.399 |
| project-open:small | 23.723 / 24.457 | 15.928 / 16.941 |
| project-open:unicode-long-line | 18.003 / 18.479 | 22.603 / 23.299 |
| project-query:small | 24.267 / 24.896 | 20.657 / 23.684 |
| project-query:unicode-long-line | 25.548 / 26.574 | 24.768 / 25.523 |
| redo:small | 14.089 / 14.18 | 3.462 / 3.469 |
| redo:unicode-long-line | 13.927 / 14.448 | 14.184 / 15.911 |
| search-close:small | 14.256 / 14.738 | 12.511 / 13.675 |
| search-close:unicode-long-line | 13.644 / 13.9 | 13.812 / 13.887 |
| search-open:small | 21.46 / 26.469 | 14.55 / 18.464 |
| search-open:unicode-long-line | 27.042 / 27.821 | 22.537 / 27.035 |
| search-query:small | 21.696 / 22.377 | 21.653 / 21.808 |
| search-query:unicode-long-line | 22.422 / 23.944 | 20.76 / 21.158 |
| tabs:small | 13.442 / 15.515 | 12.232 / 12.789 |
| tabs:unicode-long-line | 13.954 / 17.508 | 13.493 / 13.826 |
| typing:small | 7.531 / 8.399 | 4.886 / 5.777 |
| typing:unicode-long-line | 13.45 / 15.762 | 14.287 / 14.976 |
| undo:small | 13.156 / 14.681 | 3.569 / 4.657 |
| undo:unicode-long-line | 27.05 / 27.205 | 27.642 / 27.945 |

## Kiểm chứng và giới hạn

134 Rust tests, fmt, strict native/Linux x64/macOS ARM64 cross-Clippy và release
build pass. Cross-Clippy không là runtime certification. Native Windows probes
workspace/tabs/dialog/mouse/project/search/review/Agent/MCP/recovery pass; terminal
matrix 6/6 cấu hình pass. [Chi tiết lượt chạy](../diagnostics/terminal-workflow/probes/results.json).

Hai vấn đề harness được giữ trong evidence:

1. Resize observer có thể đọc status cũ rồi gửi resize tiếp, hoặc đổi geometry
   trước khi output kích thước cũ đã drain. Lượt lỗi trace-off và raw logs giữ trong
   harness-failures; bản cuối yêu cầu fresh output và drain 30 ms mỗi phía.
   Chưa chứng nhận resize dồn dập và chưa có OS scheduling profiler.
2. Search harness lặp mở destination khi ReplaceFile Windows đang commit gây
   Access denied. Fixture riêng tái lập default reader giữ handle chặn replace;
   đóng reader thì replace được. Harness cuối chờ Saved **và** hết dirty rồi mới
   so bytes. Một lượt sửa chờ Saved đơn lẻ đọc stale status cũng bị giữ là fail.
   Product không đổi safe-save policy; lỗi giữ dirty và nguyên source.

Lịch sử cold End **916.326 ms còn mở**, không suy ra đã sửa từ các lượt mới.
Không có kết quả native Linux/macOS/SSH/tmux, real disk-full/power loss hoặc RAM
peak matrix mới. Open/save/search/Agent completion không là input-to-flush.
Không có benchmark Rust/TypeScript tương đương và không có tuyên bố hệ số tốc độ.
GitHub Actions chưa kích hoạt.
