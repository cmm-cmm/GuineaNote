# Kiểm chứng CLI guinea — 10/11/2026

Đính chính sau checkpoint: harness pty_agent.py còn ghép tên executable editor
cũ từ PROFILE/SUFFIX; phần Agent của checkpoint này đã chạy binary cũ, chưa
chứng minh guinea.exe. Không đổi raw evidence lịch sử. Harness đã sửa sang
guinea và được kiểm chứng lại trong [đợt latency50](LATENCY50_IMPROVEMENTS.md).
Các probe CLI/open/save/project/recovery dùng guinea trực tiếp giữ phạm vi cũ.

Hạng mục naming độc lập với tối ưu/save/text engine. Product Guinea Note,
executable guinea.exe, Cargo package guineanote 0.1.0 giữ nguyên.

- [Manifest/hash và phạm vi](../diagnostics/cli-migration/manifest.json).
- [134 Rust tests](../diagnostics/cli-migration/guinea-migration-tests.txt).
- [Native probes](../diagnostics/cli-migration/probes/results.json), recovery repeat
  [pass](../diagnostics/cli-migration/probes/pty_recovery-repeat.txt).
- [Raw trace](../diagnostics/cli-migration/trace/report.json) và
  [trace off](../diagnostics/cli-migration/no-trace/report.json), cùng binary.

`source_sha256` của raw report dùng phạm vi Cargo files và crates/*.rs; manifest
bổ sung `full_runtime_source_sha256` gồm cả src/main.rs. Harness mới ghi rõ phạm
vi và thêm root src. Không sửa các mẫu latency hay metadata gốc của raw report;
binary hash và commit bao chứa xác định build đầy đủ.

Help/version và open/save smoke dùng guinea.exe; clipboard/paste fixture giữ
BOM/NFD/ZWJ/LF/CRLF. Workspace/project/6 cấu hình terminal/Agent/recovery pass.
Recovery probe đầu lỗi tìm tên process cũ (StopIteration, chưa kill editor);
sửa chọn chính xác tên BINARY rồi chạy lại pass. Log đầu giữ nguyên.
Config/draft schema, persistent paths, biến môi trường và Agent IPC không đổi.

fmt, native/Linux x64/macOS ARM64 strict cross-Clippy và release workspace build
pass. Chưa có runtime Linux/macOS/ARM/SSH/tmux. Chưa publish, chưa bật Actions.

## Input-to-flush warm sau migration

Nearest-rank, cùng [phương pháp](BENCHMARK_METHODOLOGY.md); không đo pixel/adapter queue.
2016 warm samples, 28 nhóm, zero dropped, tất cả nhóm warm đủ mẫu đạt trong run
này. All-phase max 22.349 ms. Không suy ra cải thiện tốc độ từ rename.

| Corpus | Operation | n | p50 ms | p95 ms | p99 ms | max ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| small | caret | 250 | 0.897 | 1.304 | 1.513 | 2.638 |
| small | delete | 124 | 0.924 | 1.112 | 1.584 | 1.773 |
| small | dialog_input | 29 | 1.137 | 1.427 | 3.205 | 3.205 |
| small | dialogs | 29 | 1.234 | 1.655 | 1.732 | 1.732 |
| small | history | 49 | 0.953 | 1.086 | 1.117 | 1.117 |
| small | mouse | 49 | 2.382 | 2.889 | 3.186 | 3.186 |
| small | project_open | 24 | 1.589 | 2.504 | 2.895 | 2.895 |
| small | project_query | 49 | 1.076 | 1.614 | 2.9 | 2.9 |
| small | resize | 74 | 3.644 | 4.576 | 4.79 | 4.79 |
| small | search_query | 99 | 3.106 | 4.346 | 5.126 | 5.126 |
| small | search_ui | 49 | 4.189 | 4.699 | 4.88 | 4.88 |
| small | tabs | 60 | 2.577 | 3.103 | 3.192 | 3.192 |
| small | typing | 124 | 0.927 | 1.126 | 1.247 | 1.5 |
| unicode-long-line | caret | 199 | 1.28 | 2.947 | 3.939 | 4.168 |
| unicode-long-line | delete | 124 | 4.052 | 19.766 | 21.727 | 22.349 |
| unicode-long-line | dialog_input | 29 | 3.7 | 4.344 | 4.532 | 4.532 |
| unicode-long-line | dialogs | 29 | 4.038 | 4.653 | 5.467 | 5.467 |
| unicode-long-line | history | 49 | 6.007 | 18.126 | 18.712 | 18.712 |
| unicode-long-line | long_line_end | 24 | 2.744 | 3.068 | 3.217 | 3.217 |
| unicode-long-line | long_line_home | 25 | 2.692 | 3.031 | 3.145 | 3.145 |
| unicode-long-line | mouse | 49 | 2.569 | 2.919 | 2.993 | 2.993 |
| unicode-long-line | project_open | 24 | 4.462 | 5.023 | 5.039 | 5.039 |
| unicode-long-line | project_query | 49 | 3.392 | 4.424 | 4.595 | 4.595 |
| unicode-long-line | resize | 74 | 3.569 | 4.421 | 4.547 | 4.547 |
| unicode-long-line | search_query | 99 | 3.172 | 4.387 | 4.618 | 4.618 |
| unicode-long-line | search_ui | 49 | 4.374 | 5.222 | 5.695 | 5.695 |
| unicode-long-line | tabs | 60 | 2.532 | 2.993 | 3.155 | 3.155 |
| unicode-long-line | typing | 124 | 3.162 | 5.428 | 6.423 | 6.967 |

Các cold category còn ít mẫu, Agent contention chưa đủ grouped internal latency,
resize dồn dập chưa chứng nhận. Cold End **916.326 ms còn mở**. Không có SLA toàn cục.
Baseline trước migration và các harness failure giữ trong [baseline](LATENCY_BASELINE.md).
