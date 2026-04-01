# Chính sách bảo mật — VnKey

## Phiên bản được hỗ trợ

| Phiên bản | Hỗ trợ |
| --------- | ------ |
| 1.0.x     | ✅      |
| < 1.0     | ❌      |

## Báo cáo lỗ hổng bảo mật

Nếu bạn phát hiện lỗ hổng bảo mật trong VnKey, **vui lòng KHÔNG mở issue công khai**.

Thay vào đó, hãy báo cáo qua:

- **GitHub Private Vulnerability Reporting**: [Báo cáo tại đây](https://github.com/marixdev/vnkey/security/advisories/new)

Vui lòng cung cấp:

1. Mô tả lỗ hổng
2. Các bước để tái hiện
3. Phiên bản VnKey và hệ điều hành bị ảnh hưởng
4. Mức độ nghiêm trọng theo đánh giá của bạn

## Quy trình xử lý

- Xác nhận nhận báo cáo trong vòng **48 giờ**
- Đánh giá và phân loại mức độ nghiêm trọng
- Phát hành bản vá và thông báo công khai

## Lưu ý về VnKey

VnKey là bộ gõ tiếng Việt chạy với quyền hạn bình thường (không yêu cầu admin/root). Trên Windows, VnKey sử dụng keyboard hook (`SetWindowsHookEx`) để xử lý phím — đây là API chuẩn của Windows cho input method.

Các file thực thi phát hành trên Windows được **ký số** thông qua [SignPath](https://signpath.io).
