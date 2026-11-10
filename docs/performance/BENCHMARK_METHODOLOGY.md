# Phương pháp đo tương tác

## Hai phép đo khác nhau

1. `input_to_flush`: start ngay khi main event loop nhận event đã decode; end sau
   khi Terminal::draw trả thành công, gồm commit/layout/render/write/flush và
   thời gian chờ poll_jobs trước frame. Không gồm độ trễ trước khi main loop nhận
   event (transport, decoder/adapter queue) và không đo pixel trên màn hình.
2. PTY end-to-end: từ gửi phím đến screen parser nhìn thấy trạng thái mới;
   gồm transport và Python observer. Báo riêng, không dùng thay cho phép đo 1.

Trace stage: input_handle, poll_jobs, cursor_layout, line_layout_paint,
frame_build, draw_total, terminal_write, terminal_flush, input_to_flush. Write
và flush là tổng thời gian gọi writer trong frame, không phải độc lập với draw.
OS scheduling và adapter queue cần profiler riêng; chưa có instrumentation riêng.

CSV ghi operation category, cold/warm, bytes hiện tại, viewport, batch size,
timestamp và dropped_samples. Không ghi phím, source, clipboard hoặc đường dẫn.
Trace bounded 4096 rows, try_send; I/O trên consumer. Dropped>0 làm run không đủ
điều kiện nghiệm thu. Thời gian clock/write instrumentation thuộc overhead trace.

Cold ở harness này là lần đầu một category trên document trong process; warm là
các lần sau. Không gọi đây là cold disk/cache OS. Phiên mới tách riêng bằng session.
Mỗi phím trong corpus thường xuyên chờ feedback trước khi gửi tiếp; paste/burst
là workload riêng, raw batch_events giữ phụ thuộc của các event dùng chung frame.

Nearest-rank percentile: sort n samples, lấy ceil(p*n)-1. Báo n/p50/p95/p99/max,
số và tỷ lệ >100 ms cho mỗi operation/phase/corpus/size bucket. Nhóm ít hơn 20
mẫu ghi insufficient_samples; không gọi p99 của vài mẫu là tail estimate đáng tin.
Các mục tiêu chỉ được pass trên nhóm đủ dữ liệu đã công bố; thiếu nhóm/OS vẫn mở.

Theo yêu cầu mới nhất, mọi mẫu tương tác >50 ms phải được ghi vào danh sách điều
tra riêng, kể cả khi percentile nhóm vẫn đạt hoặc nhóm chưa đủ mẫu. Giữ cột
over_100ms để đối chiếu lịch sử, thêm over_50ms và raw application/PTY
investigations. Background completion tách riêng. Ngưỡng 100 ms cũ đã được thay.
Nếu n<20 nhưng có mẫu >50 ms, status điều tra được ưu tiên; n vẫn không đủ để
nghiệm thu percentile. Không bỏ qua outlier vì nhóm chưa đủ mẫu.

Trace có agent_active tại main-loop receipt; đây là state/lease đang hoạt động,
không phải bằng chứng CPU worker thực thi đồng thời ở từng nanosecond. So sánh
context Agent active/idle riêng, không gộp percentile để nghiệm thu.

Mỗi run ghi OS, terminal, TERM/COLORTERM, profile, git HEAD, dirty source hash,
binary SHA256, trace on/off và fixture sizes. Chạy tuần tự, không build/audit cùng
lúc đo. Dữ liệu raw được giữ cả khi mục tiêu không đạt. Trước/sau phải cùng phép
đo và corpus; số đo stage cũ không so như thể là input-to-flush baseline.

Run trace on/off cùng corpus để quan sát overhead PTY; chỉ trace-on có số nội bộ.
Open/save/search completion đo riêng. Kiểm tra Unicode/NFD/ZWJ/mixed EOL,
readonly/AI, crash recovery và cleanup trước khi nhận cải thiện hiệu suất.
