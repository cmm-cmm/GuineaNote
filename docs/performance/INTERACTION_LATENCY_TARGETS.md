# Mục tiêu độ trễ tương tác hiện hành

Người dùng yêu cầu áp dụng từ 10/11/2026 theo đặc tả Notion:
https://app.notion.com/p/41427da1dc0881309c02cf504b7c59dd

- Input-to-terminal-flush: p50 <=10 ms, p95 <=20 ms, p99 <=50 ms.
- Tính riêng từng nhóm thao tác, corpus, cold/warm; không dùng percentile hỗn hợp để nghiệm thu.
- **Mọi phản hồi >50 ms phải điều tra**, kể cả khi percentile cả nhóm vẫn đạt.
- Idle gần 0 CPU; không redraw hoặc busy polling khi không đổi trạng thái.
- Open/save/scan/review/Agent chạy nền, giới hạn và có trạng thái rõ ràng.
- Không giảm correctness, Unicode, Undo/Redo, EOL, file safety hoặc recovery để đạt số đẹp.

Đây là mục tiêu được người dùng yêu cầu triển khai, chưa phải SLA đã nghiệm thu.
387/387 mẫu cũ dưới 100 ms và p95/p99 hỗn hợp 31.525/34.637 ms không chứng minh
đạt mục tiêu mới. Cold End 916.326 ms giữ nguyên và vẫn chưa rõ nguyên nhân.

GitHub Actions chưa kích hoạt theo chỉ đạo trực tiếp của người dùng; phần CI
trong roadmap Notion được hoãn. Các checks/harness chạy local trên từng máy.

## Điều chỉnh ưu tiên theo yêu cầu trực tiếp

Người dùng yêu cầu không cần kiểm chứng SSH với Ubuntu trong đợt tiếp theo;
ưu tiên kiểm chứng độ trễ. Ubuntu/SSH/tmux không phải dependency hoặc gate
nghiệm thu của đợt latency này. Không dành công việc chuẩn bị môi trường remote
cho đợt này. Các runtime chưa đo vẫn được ghi là chưa kiểm chứng.

Thứ tự kế hoạch mới:

1. Mở rộng baseline local trên Windows/ConPTY: typing/delete/caret, long-line
   Home/End, history, tabs/dialogs, project/search, mouse/resize; tách cold/warm.
2. Điều tra cold End 916.326 ms; bổ sung phân đoạn decode/queue khi khả thi và
   đối chiếu PTY/OS scheduling. Không đổi định nghĩa input_to_flush cũ hoặc gộp
   các metric khác nhau để tạo so sánh before/after.
3. Đo input-to-flush khi Agent/MCP, search/review/open/save chạy nền; ưu tiên
   profile delete/Undo trên dòng Unicode 10 MiB vì p95 hiện sát 20 ms.
4. Chỉ tối ưu sau khi có trace; chạy lại cùng corpus, binary/profile/môi trường
   đối chiếu và trace on/off. Theo dõi RSS/idle CPU cùng tác động latency.
5. Giữ raw samples/failure, hash, số mẫu và giới hạn suy luận; cập nhật baseline,
   docs và Notion. Các UI/features mới xếp sau đợt kiểm chứng này.

Đề xuất lấy ít nhất 100 phiên mới cho first-use cases trọng tâm và 1.000 mẫu
warm/nhóm trọng tâm, báo cold riêng; số mẫu không tự bảo đảm độ tin cậy p99.
Nhóm thiếu dữ liệu giữ trạng thái chưa đủ bằng chứng. Tất cả mẫu >50 ms phải
được lưu và điều tra. Không đánh đổi dữ liệu, Unicode, history hoặc recovery.

Ngưỡng 50 ms thay cho ngưỡng điều tra 100 ms trước đây theo yêu cầu trực tiếp
mới nhất. Các báo cáo lịch sử không bị sửa số liệu; cần phân loại lại từ raw
samples khi đối chiếu chính sách mới. Background completion được báo riêng,
không bị đánh đồng với phản hồi tương tác.
