# Checkpoint tạm dừng — tối ưu layout ASCII

**Trạng thái: tạm dừng theo yêu cầu người dùng, 10/11/2026.** Chỉ chốt kết quả
và commit; không tiếp tục triển khai/đo mới sau yêu cầu tạm dừng. Các probe cuối
đã kết thúc khi kiểm tra trạng thái. Commit bao chứa tài liệu xác định checkpoint.

## Quá trình

1. Review dev2 phát hiện bốn lỗi P1: signal Unix đi qua discard recovery, Unicode
   tại checkpoint, Unicode qua ranh giới chunk và first-End vượt 50 ms. Người dùng
   yêu cầu hủy thay đổi/xóa dev2; đã thực hiện, trở về dev fe46835 và build lại.
2. Bắt đầu tối ưu độc lập trên dev; không khôi phục source dev2. Baseline mới
   phát hiện quét lại toàn dòng ASCII khi tính caret và viewport cuối dòng.
3. Dùng lại cache layout hiện có cho ASCII dài, giữ nguyên engine Unicode. Tách
   tái dùng khoảng trống ASCII đầu chuỗi khỏi việc giữ index; cập nhật cell shift
   trước tab đầu tiên theo tab stops và giữ fallback cho trường hợp khác.
4. Candidate đầu không còn >50 ms nhưng End trên dòng có tab p95 21.871 ms chưa
   đạt mục tiêu 20 ms. Đã điều tra và bổ sung giữ checkpoint cho ASCII trước tab.
5. Thêm guard không tạo full ASCII layout copy mới cho dòng >16 MiB. Thêm các
   test Unicode/checkpoint/chunk, tab stops, snapshot, CRLF, Undo/Redo và guard.
6. Thêm microbenchmark ASCII viewport, PTY harness riêng và thống kê send/wait/
   observer CPU. Bộ kiểm chứng cuối hoàn tất trước khi ghi nhận tạm dừng.

## Kết quả đã xác minh

140 Rust tests, strict native Clippy và release workspace build pass. Các probe
cuối ASCII trace on/off, search, general, Agent, recovery, terminal matrix, cold
navigation, Agent contention và general trace on/off đều exit 0.
[Lệnh/kết quả](../diagnostics/ascii-layout/regression-final/results.json).
Tất cả các phép đo là native Windows/ConPTY release. Không kiểm chứng Ubuntu/SSH,
không bật GitHub Actions. Mọi phản hồi >50 ms phải điều tra.

| Dòng 10 MiB, nhóm warm | n trước/sau | p95 trước ms | p95 sau ms | p99 sau ms | max sau ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| ascii / long_line_end | 29/29 | 78.776 | 2.131 | 4.346 | 4.346 |
| ascii / caret | 59/59 | 75.667 | 1.78 | 1.797 | 1.797 |
| ascii-tabs / long_line_end | 29/29 | 112.062 | 2.169 | 2.382 | 2.382 |
| ascii-tabs / caret | 59/59 | 121.407 | 1.955 | 2.249 | 2.249 |

Baseline ASCII: **117 input-to-flush samples >50 ms**, 122 PTY observations >50 ms.
Bản cuối cùng: **364 input-to-flush samples**, max 4.346 ms; không có mẫu >50 ms ở
cả trace nội bộ và PTY. Nearest-rank percentile với n nhỏ là mô tả, không phải
chứng nhận tail dài hạn. Raw [trước](../diagnostics/ascii-layout/before/report.json),
[candidate](../diagnostics/ascii-layout/intermediate-cache/report.json),
[sau](../diagnostics/ascii-layout/final/ascii-navigation/report.json),
[trace off](../diagnostics/ascii-layout/final/ascii-navigation-off/report.json).

General corpus cuối: 2050 samples, max 19.42 ms.
Cold navigation: 26 phiên; Agent contention: 10 phiên.
Các dữ liệu báo riêng theo corpus/phase/context; không gộp percentile. Một số
nhóm ít mẫu vẫn insufficient; zero dropped ở các trace cuối.

## RAM và giới hạn

Cache thêm bản sao text cho dòng ASCII dài; đánh đổi latency/RAM, không phải
tối ưu RAM. Quan sát RSS và peak working set theo bytes:

| Fixture | RSS mở trước | RSS mở sau | RSS sample max sau | OS peak sau |
| --- | ---: | ---: | ---: | ---: |
| ascii | 20602880 | 32391168 | 32784384 | 36835328 |
| ascii-tabs | 20611072 | 33710080 | 33878016 | 36855808 |

Các target quota cache cũ vẫn áp dụng; capacity, metadata, active working set và
worker là phần thêm, không có hard RSS bound. Guard >16 MiB chặn **tạo copy ASCII
mới** ở lần lookup, không cam kết giới hạn tăng trưởng mọi cache/khả năng file lớn.
Phạm vi đo tương tác là tới 10 MiB. Open/startup completion báo riêng trong raw.

## Điều tra và việc còn mở lúc dừng

- Giữ mẫu search PTY **55.417 ms**, trace nội bộ toàn lượt ≤6.655 ms;
  [raw điều tra](../diagnostics/ascii-layout/investigation-search-55ms/report.json).
  Chưa định lượng đủ adapter queue/transport/observer để chốt root cause mẫu cũ.
  Lượt cuối search PTY max 47.035 ms, internal max 12.827 ms; observer CPU là số đo process_time
  có độ phân giải thô trên Windows. Passing repeat không tự đóng điều tra.
- Cold End lịch sử **916.326 ms** vẫn mở; không suy ra đã sửa từ sample mới.
- Chưa chạy soak RAM dài hạn/nhiều tab hoặc lặp test song song nhiều lượt trong
  checkpoint này. Có một suite 140 tests pass; không tuyên bố hết mọi flaky test.
- Microbenchmark core có CSV before/after; hash binary before của example chưa
  thu, nên chỉ hỗ trợ chẩn đoán, không dùng thay terminal measurements có hash.
- Khi tiếp tục: đối chiếu checkpoint/hash, ưu tiên profiling queue/decode và
  observer cho search, mở rộng cold/warm samples, đánh giá trade-off RAM; giữ
  nguyên Unicode/file safety/history/recovery và không đưa SSH/CI thành gate.

[Manifest và checksum raw evidence](../diagnostics/ascii-layout/manifest.json).
