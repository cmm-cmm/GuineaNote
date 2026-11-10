# Mục tiêu độ trễ tương tác hiện hành

Người dùng yêu cầu áp dụng từ 10/11/2026 theo đặc tả Notion:
https://app.notion.com/p/41427da1dc0881309c02cf504b7c59dd

- Input-to-terminal-flush: p50 <=10 ms, p95 <=20 ms, p99 <=50 ms.
- Tính riêng từng nhóm thao tác, corpus, cold/warm; không dùng percentile hỗn hợp để nghiệm thu.
- >100 ms là cảnh báo/điều tra, không phải lời hứa tuyệt đối cho mọi thao tác.
- Idle gần 0 CPU; không redraw hoặc busy polling khi không đổi trạng thái.
- Open/save/scan/review/Agent chạy nền, giới hạn và có trạng thái rõ ràng.
- Không giảm correctness, Unicode, Undo/Redo, EOL, file safety hoặc recovery để đạt số đẹp.

Đây là mục tiêu được người dùng yêu cầu triển khai, chưa phải SLA đã nghiệm thu.
387/387 mẫu cũ dưới 100 ms và p95/p99 hỗn hợp 31.525/34.637 ms không chứng minh
đạt mục tiêu mới. Cold End 916.326 ms giữ nguyên và vẫn chưa rõ nguyên nhân.

GitHub Actions chưa kích hoạt theo chỉ đạo trực tiếp của người dùng; phần CI
trong roadmap Notion được hoãn. Các checks/harness chạy local trên từng máy.
