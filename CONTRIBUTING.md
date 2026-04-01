# Đóng góp cho VnKey

Cảm ơn bạn quan tâm đến VnKey! Dưới đây là hướng dẫn để đóng góp cho dự án.

## Báo lỗi (Bug Report)

Mở [issue mới](https://github.com/marixdev/vnkey/issues/new?template=bug_report.yml) với các thông tin:

- Hệ điều hành và phiên bản (Windows 11, macOS 15, Ubuntu 24.04, …)
- Phiên bản VnKey
- Kiểu gõ đang dùng (Telex, VNI, …)
- Các bước để tái hiện lỗi
- Kết quả mong đợi vs kết quả thực tế

## Đề xuất tính năng

Mở [issue mới](https://github.com/marixdev/vnkey/issues/new?template=feature_request.yml) mô tả tính năng bạn muốn và lý do.

## Gửi Pull Request

1. **Fork** repo và tạo branch từ `master`
2. Đặt tên branch rõ ràng: `fix/ten-loi` hoặc `feat/tinh-nang-moi`
3. Đảm bảo code biên dịch thành công và test pass:
   ```bash
   cd vnkey-engine && cargo test
   ```
4. Mỗi PR nên tập trung vào **một vấn đề duy nhất**
5. Viết commit message rõ ràng theo format:
   ```
   fix: mô tả ngắn gọn
   feat: mô tả ngắn gọn
   ```

## Kiến trúc dự án

```
vnkey-engine/    Rust     Core engine + C FFI (staticlib)
vnkey-windows/   Rust     Ứng dụng Windows (Win32 + WebView2)
vnkey-macos/     Obj-C    Input method cho macOS (IMKit)
vnkey-fcitx5/    C++      Fcitx5 addon cho Linux
vnkey-ibus/      C        IBus engine cho Linux
```

- **vnkey-engine** là thư viện dùng chung, export qua C FFI. Thay đổi engine ảnh hưởng tất cả nền tảng.
- Mỗi platform module build riêng, link với `libvnkey_engine.a`.

## Môi trường phát triển

### Yêu cầu chung
- Rust toolchain (stable)
- CMake ≥ 3.16 (cho Linux builds)

### Windows
```powershell
cd vnkey-windows
cargo build --release
```

### Linux (Fcitx5)
```bash
cd vnkey-engine && cargo build --release && cd ..
cd vnkey-fcitx5 && bash scripts/build.sh package
```

### Linux (IBus)
```bash
cd vnkey-engine && cargo build --release && cd ..
cd vnkey-ibus && mkdir build && cd build
cmake .. -DCMAKE_BUILD_TYPE=Release && make -j$(nproc)
```

### macOS
```bash
cd vnkey-macos && ./build.sh install
```

## Quy tắc ứng xử

Dự án tuân theo [Quy tắc ứng xử](CODE_OF_CONDUCT.md). Bằng việc đóng góp, bạn đồng ý tuân thủ quy tắc này.

## Giấy phép

VnKey sử dụng [GPL-3.0](LICENSE). Mọi đóng góp đều được phát hành theo giấy phép này.
