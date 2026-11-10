# 02 — Text engine và kiến trúc buffer

Ngày: **10/11/2026**. Kết luận đề xuất: **tiếp tục Ropey cho MVP**, tối ưu các layer bao quanh và đo workload lớn trước khi chọn backend thứ hai. Chưa có bằng chứng GuineaNote cần viết lại text engine.

## Hiện trạng đã kiểm tra

`guinea-core` dùng Ropey **1.6.1**, cấu hình `default-features = false, features = ["simd"]`. `TextBuffer` có scalar/byte conversion, `chunk_at_byte`, `visit_chunks`, cùng `line/slice/text` trả `String`. `RopeyBuffer` giữ Rope private; TUI không chạm Rope trực tiếp. Tuy nhiên `Document`, history snapshot và API snapshot vẫn dùng kiểu `RopeyBuffer` cụ thể. Do đó **đã có boundary tốt nhưng chưa thay backend chỉ bằng một cấu hình**. Nguồn nội bộ: [buffer.rs](../../crates/guinea-core/src/buffer.rs), [document.rs](../../crates/guinea-core/src/document.rs), [view.rs](../../crates/guinea-core/src/view.rs).

Ropey nhận UTF-8 và index theo Unicode scalar, không tự cung cấp caret theo grapheme hoặc terminal cell. GuineaNote đã có module tọa độ và grapheme riêng, cần giữ. Nguồn: [Ropey](https://docs.rs/ropey/1.6.1/ropey/), [coordinates.rs](../../crates/guinea-core/src/coordinates.rs), [graphemes.rs](../../crates/guinea-core/src/graphemes.rs).

## Ký hiệu và cách đọc độ phức tạp

`N`: bytes văn bản; `m`: bytes chèn/xóa; `P`: số piece; `d`: khoảng cách gap phải di chuyển; `B`: kích thước page; `K`: số page; `q`: số piece/page bị xóa. Unicode metadata có thể cần thêm quét trong chunk. Các bound dưới đây là phân tích cấu trúc tiêu biểu, không phải số đo tốc độ. “Rope”, “Piece Tree” hoặc “Paged Buffer” không xác định duy nhất một thuật toán; persistent/mutable và cách indexing thay đổi chi phí.

| Cấu trúc | Insert | Delete | Undo/Redo | RAM/phân mảnh | Search và Unicode |
| --- | --- | --- | --- | --- | --- |
| Rope cân bằng, chunk UTF-8 | Thường `O(log N + m)` | Theo implementation; Ropey công bố `O(log N + m)` | Root snapshot chia sẻ; sửa gây copy-on-write | `O(N)` text + tree/slack; nhiều allocation nhỏ | Quét chunks, xử lý match qua biên; cần metadata scalar/line |
| Piece Table dạng vector | Locate/update thường `O(P) + O(m)` | `O(P + q)` khi không copy phần xóa | Giữ descriptor/root cũ hoặc log edit; add buffer giữ bytes cũ | Original + append-only add + descriptors; piece tăng theo edit | Full scan `O(N + P)` với matcher streaming; Unicode metadata riêng |
| Piece Tree cân bằng | `O(log P + m)` và index phần mới | `O(log P + q)` tùy thao tác cây | Snapshot rẻ chỉ nếu persistent; mutable tree cần log/copy riêng | Bytes base/add + `O(P)` nodes/index; cần coalesce/compaction | Offset/line lookup nhờ subtree metadata, match xuyên pieces |
| Gap Buffer | `O(d + m)`, gần gap amortized rẻ | Gần gap thường di chuyển biên; trả nội dung xóa thêm `O(m)` | Log chứa text xóa/chèn; snapshot naive `O(N)` | Contiguous `N + gap capacity`; reallocate khi gap hết | Quét hai đoạn dễ; di chuyển gap/offset phải tôn trọng UTF-8 |
| Paged Buffer | Lookup/index + sửa/split page, thường `O(log K + B + m)` nếu có cây | Phụ thuộc số page ảnh hưởng, index và cách reclaim | Page COW hoặc journal; cache eviction cần backing store | Có thể cap resident cache; metadata/add/history vẫn tăng | Search toàn file vẫn phải đọc dữ liệu; decode/grapheme qua page |
| Memory mapping | Không phải thuật toán edit có insert/delete tự nhiên | Không có thao tác co file an toàn tự động | Phải bổ sung overlay/pieces/journal | Virtual address ≠ RSS; page faults và dirty pages có chi phí | Cho phép đọc theo nhu cầu; vẫn cần Unicode/line index |

Bound Ropey lấy từ [API Rope 1.6.1](https://docs.rs/ropey/1.6.1/ropey/struct.Rope.html); Piece Tree đối chiếu [thiết kế VS Code](https://code.visualstudio.com/blogs/2018/03/23/text-buffer-reimplementation) và [source Fresh](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor-core/src/model/piece_tree.rs). Các hàng còn lại là suy luận thuật toán với giả định trong bảng, không gán bound đó cho một editor chưa đọc source.

## Ropey: phù hợp MVP nhưng không miễn phí

Ropey document công bố clone `O(1)` bằng chia sẻ storage, phù hợp worker đọc snapshot. Việc insert/remove vẫn có chi phí theo text ảnh hưởng; copy text ra `String` là tuyến tính. Một root clone rẻ không có nghĩa `Document::clone()` rẻ: GuineaNote derive Clone cho cả vector Undo/Redo. Nó cũng không bảo đảm thời gian drop khi lần cuối giải phóng nhiều node. Nguồn: [API Rope](https://docs.rs/ropey/1.6.1/ropey/struct.Rope.html), source nội bộ nêu trên.

**Vấn đề cần đo trước:**

- `start_save` và legacy `start_compute` clone toàn `Document` ngay trên UI thread; chi phí có thể tăng với số transaction dù nội dung chỉ 1 MB.
- Save hiện materialize `String`, rồi thêm `Vec<u8>` có BOM: có hai bản bytes tạm ngoài Rope. Search/review cũng có full-text snapshot trên worker. Chuyển ra worker giảm UI block, không giảm peak RAM.
- `TextBuffer::delete` trả text đã xóa; `replace_range` còn so sánh slice trước đó. Với selection rất lớn cần đo số bản copy và thời gian cả handler.
- Layout cache của dòng Unicode dài giữ một bản owned line cùng sparse checkpoints. Chuẩn bị trước dòng đầu <=10 MiB là tối ưu có ích, không bao phủ dòng thứ hai rất dài hay buffer do Agent tạo.
- History không có quota; shared snapshots vẫn có thể giữ toàn bộ bytes cũ qua nhiều phiên bản.

Đây là đường đi mã đã thấy, **chưa chứng minh chúng gây mẫu outlier 916.326 ms**.

## Piece Table và Piece Tree: lợi ích đi cùng chi phí

Piece Table giữ original immutable và add buffer append-only; descriptor chỉ vùng hiện hữu. Piece Tree thêm tree/index để tránh tìm piece tuyến tính. Không cần một object cho mỗi dòng; đây là bài học từ VS Code. Tuy vậy persistent tree, line index, UTF-8 validation, cursor mapping, compaction và snapshot isolation đều phải xây hoặc thẩm định. Piece Tree vẫn có thể load toàn bộ file: nó không tự động thành backend RAM giới hạn.

Fresh kết hợp tree với loaded/unloaded storage. Đó là hai quyết định: **cấu trúc edit** và **chính sách resident I/O**. Muốn giữ bytes gốc trên disk phải bảo vệ base version khỏi external modification, overwrite và unlink; chỉ nhớ đường dẫn là chưa đủ cho snapshot đúng. Nguồn: [buffer Fresh](https://github.com/sinelaw/fresh/blob/a1ce79df50b0642bdb6badd9dc9eb010c1e133cf/crates/fresh-editor-core/src/model/buffer/mod.rs).

**Đề xuất:** không đổi sang Piece Tree ở MVP. Sau khi streaming save/open và memory quotas vẫn chưa đủ, làm prototype cô lập và chạy chung semantic suite. Chỉ chọn nếu có cải thiện đáng kể ở 100 MB–1 GB, random edits, long lines và peak RSS; không chọn chỉ vì startup trông nhanh.

## Gap Buffer: đơn giản nhưng không hợp ưu tiên hiện tại

Gap Buffer phù hợp typing tập trung ở một chỗ, locality tốt và metadata ít. Chuyển gap xa, random editing, replace-all hoặc nhiều caret làm tăng chi phí dịch bytes. Undo bằng delta có thể gọn, nhưng immutable snapshot cho worker không còn rẻ nếu copy cả buffer. Như vậy chuyển từ Ropey sang gap sẽ đánh mất một cơ chế đang phục vụ search, review, save và Agent.

Manual GNU Emacs 30.2 mô tả gap chỉ di chuyển khi insert/delete, vì thế lần edit xa vị trí trước có thể chậm hơn. Endpoint HTML không đọc được; đã đọc section Buffer Gap trong [source manual cố định](https://github.com/emacs-mirror/emacs/blob/636f166cfc86aa90d63f592fd99f3fdd9ef95ebd/doc/lispref/buffers.texi). Đây là mô tả cơ chế, không phải số đo Emacs hay bằng chứng Nano dùng cùng backend.

## Paged Buffer và memory mapping

Paged Buffer có cache eviction, prefetch, backing store immutable và index phân cấp có thể giữ working set nhỏ. Nhưng chi phí `Go to line` chính xác phải trả ở đâu đó: scan index trước hoặc xây từng phần rồi báo chưa sẵn sàng. Search toàn file không thể biết tất cả kết quả khi chưa đọc phần chưa index.

Memory mapping chỉ là cơ chế truy cập bytes. File truncate/modify đồng thời có thể gây lỗi nghiêm trọng; API file-backed của memmap2 có điều kiện `unsafe`. Không dùng writable mapping để sửa file gốc: trái với chính sách safe save và chưa có cách insert/delete hợp lý. Ngay cả copy-on-write mapping cũng cần thỏa safety contract của dữ liệu backing. Nguồn: [memmap2, Safety](https://docs.rs/memmap2/latest/memmap2/struct.MmapOptions.html), trang đã đọc hiển thị 0.9.12.

**Đề xuất mặc định:** prototype read-only paged với I/O an toàn và cache trước; chỉ khảo sát mmap nếu profiling cho thấy cần thiết và có RFC về vòng đời file, truncate, locking, Windows replacement và unsafe boundary. Giữ `unsafe_code = forbid` ở ứng dụng.

## Đánh giá theo kích thước

Đơn vị dưới đây là **MB thập phân** theo yêu cầu; corpus cũ là **10 MiB = 10.485.760 bytes**. Các nhận định lớn hơn corpus là dự báo kỹ thuật, không phải hỗ trợ đã chứng nhận.

| File | Chứng cứ GuineaNote hiện có | Đánh giá Ropey | Hướng đề xuất |
| --- | --- | --- | --- |
| 1 MB | Nằm trong phạm vi corpus đã lưu; không có benchmark mới riêng 1 MB ở báo cáo này | Phù hợp; overhead UI/history có thể quan trọng hơn tree | Backend mặc định |
| 10 MB | Corpus tới 10 MiB; 451 transitions, một failure >100 ms | Tiếp tục; xử lý long-line/copy/history | Phạm vi interactive cần hardening |
| 100 MB | Chưa có evidence native hiện tại đủ để cam kết | Resident Rope khả thi tùy RAM, app có bản copy tạm | Cảnh báo mở, đo RSS và cancellation trước |
| 500 MB | Chưa kiểm chứng | Không suy từ `O(log N)` sang đủ RAM/latency | Paged read-only prototype, index chính xác tiến dần |
| 1 GB | Chưa kiểm chứng | Bản thân Ropey có thiết kế cho text lớn; toàn app chưa có bằng chứng | Không quảng cáo edit 1 GB; qua capability mode và safety gate |

Không đặt số RAM giả như “Ropey luôn 1,2×” hay “mmap gần 0 RAM”. Mô hình đo cần tính **base text + tree + retained history + cache + workers + materialized output**, cùng peak working set và virtual mappings.

## Dòng rất dài là một trục kiểm thử riêng

File 10 MB một dòng và file 10 MB nhiều dòng là workload khác nhau. Engine có thể tìm offset nhanh nhưng grapheme/cell width, tabs, syntax state và wrap cần đọc ngữ cảnh. Một grapheme cluster bệnh lý cũng có thể rất dài; không cắt sai cluster để ép bound. ASCII fast path phải có điều kiện kiểm chứng; Unicode path có budget, cache checkpoint và báo pending khi phải chuẩn bị lâu.

Ma trận bắt buộc: ASCII; tiếng Việt NFC/NFD; CJK; ZWJ/variation selector; regional indicators; chuỗi combining marks dài; tabs đầu/giữa/cuối; CRLF qua chunk; dòng dài ở đầu/giữa/cuối; lần đầu End/Home/drag và lần dùng cache. Đo cả sau Undo, reload, Agent edit và resize. Không đo chỉ cursor gần đầu dòng.

## Boundary cho backend thứ hai

Không cần generic hóa toàn app ngay. Đề xuất tiến từng bước:

1. **Snapshot không kèm history:** immutable read snapshot có id/revision, bytes/chunks, line ranges và metadata cần thiết. Worker không nhận bản clone lịch sử.
2. **API range/chunk là đường chính:** giữ `String` cho clipboard/dialog nhỏ; không để renderer/search/I/O bắt buộc full `text()`.
3. **Edit result có delta:** ranges cũ/mới, revision và boundary validity để invalidate cache; không gắn trực tiếp với cấu trúc Rope.
4. **Capabilities rõ:** editable, exact line count, exact scalar conversion, recovery support; giá trị chưa biết dùng kiểu `Pending/Unavailable`, không trả số ước lượng trá hình.
5. **Chọn storage sau bằng enum hoặc trait:** `Ropey` và backend lớn có snapshot riêng sau một API đọc; chọn dựa trên prototype, không thiết kế plugin ABI buffer.

UI vẫn dùng command + `DocumentId/ViewId`, snapshot và viewport; tránh để widget phụ thuộc variant backend. Paged I/O dùng request/response, không đọc disk chặn trong `line_window`. API trả `Pending` yêu cầu UI biết loading state, nhưng không cần viết lại cách bố trí UI.

## Chi phí chuyển đổi và cổng quyết định

| Phương án | Phạm vi ảnh hưởng | Chi phí tương đối | Quyết định |
| --- | --- | --- | --- |
| Giữ Ropey, bỏ copy thừa, quota/cache | Core/I/O/workers và test | Vừa | Làm trước |
| Đổi ngay toàn bộ sang Piece Tree | Edit/history/search/Unicode/I/O/Agent/review | Rất cao, nguy cơ regression dữ liệu | Không chọn cho MVP |
| Thêm paged read-only | Snapshot API, index, cache, viewport, open policy | Cao nhưng scope giới hạn | Prototype sau hardening |
| Paged editable | Thêm overlay/history/recovery/save/snapshot isolation | Rất cao | Chỉ sau read-only có evidence |

Trước migration: property tests so với reference String; exact bytes BOM/mixed ending; grapheme mapping; Undo/Redo branching; revision conflict; lỗi đọc/ghi và crash; PTY native trên ba OS. Chỉ thông qua nếu prototype thắng mục tiêu được định nghĩa trước mà không mất tính chính xác. Chưa có kết quả như vậy trong nghiên cứu này.
