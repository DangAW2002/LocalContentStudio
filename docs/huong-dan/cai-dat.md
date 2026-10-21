# Hướng dẫn cài đặt và nâng cấp Local Content Studio

Tài liệu này hướng dẫn chi tiết cách tải về, kiểm tra tính toàn vẹn, cài đặt, thiết lập lần đầu và nâng cấp Local Content Studio trên hệ điều hành Windows.

---

## 1. Yêu cầu hệ thống

Trước khi cài đặt, hãy đảm bảo máy tính của bạn đáp ứng các yêu cầu sau:

- **Hệ điều hành**: Windows 10 (bản 64-bit) hoặc Windows 11 (64-bit).
- **Trình điều khiển WebView2**: Microsoft Edge WebView2 Runtime. Hầu hết các máy Windows 10/11 cập nhật gần đây đều đã có sẵn thành phần này. Nếu chưa có, bạn có thể tải miễn phí từ trang chủ Microsoft: <https://developer.microsoft.com/en-us/microsoft-edge/webview2/>.
- **Dung lượng ổ đĩa**: Tối thiểu 500 MB trống cho bộ cài và tệp nhị phân ứng dụng, cộng thêm dung lượng lưu trữ cho các dự án nội dung và video của bạn.

---

## 2. Tải về và kiểm tra mã băm SHA-256

Để đảm bảo tệp cài đặt tải về nguyên vẹn và an toàn, bạn nên kiểm tra mã băm SHA-256 của tệp cài:

1. Tải tệp cài đặt `Local Content Studio_0.2.0_x64-setup.exe` và tệp đối soát `SHA256SUMS.txt`.
2. Mở **PowerShell** trên máy tính (bấm phím `Windows`, gõ `powershell` và nhấn `Enter`).
3. Điều hướng tới thư mục chứa tệp cài và chạy lệnh:
   ```powershell
   Get-FileHash -Algorithm SHA256 '.\Local Content Studio_0.2.0_x64-setup.exe'
   ```
4. So sánh kết quả mã băm hiển thị với chuỗi mã băm trong tệp `SHA256SUMS.txt`. Nếu hai chuỗi ký tự trùng khớp hoàn toàn, tệp cài đặt an toàn để sử dụng.

---

## 3. Cảnh báo Windows SmartScreen (Bộ cài chưa ký số)

Do Local Content Studio là phần mềm mã nguồn mở cá nhân và **chưa mua chứng chỉ ký số thương mại Authenticode** (để tránh phát sinh chi phí duy trì hàng năm), Windows SmartScreen có thể chặn tệp và hiển thị cảnh báo màu xanh:

> **Windows protected your PC** *(Trình bảo vệ Windows đã bảo vệ PC của bạn)*  
> *Microsoft Defender SmartScreen prevented an unrecognized app from starting...*

**Cách thực hiện để tiếp tục cài đặt:**
1. Bấm vào dòng chữ **More info** *(Thông tin thêm)* ngay dưới đoạn thông báo cảnh báo.
2. Nút **Run anyway** *(Vẫn chạy)* sẽ xuất hiện ở góc dưới bên phải.
3. Bấm **Run anyway** để khởi chạy trình cài đặt.

---

## 4. Quá trình cài đặt

1. Trình cài đặt hoạt động theo cơ chế **người dùng hiện tại (currentUser)**, cài đặt ứng dụng vào thư mục riêng của bạn:  
   `%LOCALAPPDATA%\Programs\Local Content Studio`
2. Bạn **không cần quyền Quản trị viên (Administrator)** để cài đặt hay cập nhật.
3. Nhấn **Next** và chờ quá trình sao chép tệp tin hoàn tất trong vài giây.
4. Một biểu tượng phím tắt **Local Content Studio** sẽ được tự động tạo trên màn hình Desktop và trong Start Menu.

---

## 5. Thiết lập lần đầu (Kiểm tra công cụ ngoài)

Local Content Studio theo tôn chỉ thiết kế minh bạch: **không đóng gói sẵn các phần mềm nặng của bên thứ ba** (như Node.js, Chromium, FFmpeg). Thay vào đó, ứng dụng sẽ quét các công cụ đã có trên hệ thống của bạn:

Khi mở ứng dụng lần đầu, bạn hãy vào mục **Tools & Cài đặt**:

### a. Trợ lý AI (Engine)
Studio hỗ trợ phối hợp làm việc với Claude Code hoặc agy CLI:
- **Claude Code**: Yêu cầu cài đặt [Node.js](https://nodejs.org/), sau đó mở terminal chạy lệnh:
  ```powershell
  npm install -g @anthropic-ai/claude-code
  ```
- **agy (Google Antigravity CLI)**: Cài đặt và cấu hình theo hướng dẫn của Antigravity.

### b. Xử lý Đa phương tiện (FFmpeg)
FFmpeg là công cụ bắt buộc để Studio dựng video, cắt ghép âm thanh và chuẩn hóa âm lượng chuẩn YouTube (-14 LUFS):
- Bạn có thể cài đặt cực nhanh qua Windows Package Manager:
  ```powershell
  winget install Gyan.FFmpeg
  ```
- Hoặc tải bộ nén từ trang chủ <https://ffmpeg.org/download.html>, giải nén và thêm thư mục chứa `ffmpeg.exe` vào biến môi trường `PATH` của máy.
- Sau khi cài đặt, vào **Tools & Cài đặt → Kiểm tra môi trường** để bấm quét lại.

---

## 6. Nâng cấp từ phiên bản 0.1.0

Nếu bạn đang sử dụng phiên bản 0.1.0 và muốn nâng cấp lên 0.2.0:

1. Đóng ứng dụng Local Content Studio phiên bản cũ đang chạy.
2. Chạy tệp cài đặt `Local Content Studio_0.2.0_x64-setup.exe` phiên bản mới. Trình cài đặt sẽ tự động cập nhật các tệp chương trình mà không ảnh hưởng tới dữ liệu cá nhân của bạn.
3. **Cơ chế sao lưu tự động khi chuyển đổi dữ liệu**:
   - Khi bạn mở một Workspace dự án cũ bằng bản 0.2.0, hệ thống sẽ phát hiện cấu trúc cơ sở dữ liệu cũ.
   - Trước khi thực hiện nâng cấp bảng dữ liệu, Studio **tự động tạo một bản sao lưu an toàn** ngay tại:
     ```
     <thư-mục-workspace>\.studio\studio.db.<timestamp>.bak
     ```
   - Tất cả các cuộc hội thoại, các mốc khôi phục (checkpoints), tệp cấu hình thương hiệu và các tệp video đã render đều được giữ nguyên 100%.

---

## 7. Gỡ cài đặt (Uninstall) và Dọn dẹp Dữ liệu

Nếu bạn muốn gỡ cài đặt Local Content Studio:

1. Vào **Windows Settings** (phím `Windows + I`) → **Apps** → **Installed apps**.
2. Tìm **Local Content Studio**, bấm vào dấu ba chấm `...` và chọn **Uninstall** *(Gỡ cài đặt)*.
3. Để bảo vệ người dùng tránh mất mát sản phẩm sáng tạo ngoài ý muốn, trình gỡ cài đặt **sẽ không bao giờ tự ý xoá các dự án nội dung** của bạn.

Nếu muốn xoá sạch hoàn toàn mọi tệp tin còn sót lại trên máy:
- **Cấu hình ứng dụng cục bộ**: Xoá thư mục `%LOCALAPPDATA%\LocalContentStudio`
- **Dữ liệu đăng nhập và kho lưu trữ tạm**: Xoá thư mục `%APPDATA%\LocalContentStudio`
- **Các thư mục Workspace của bạn**: Tự xoá các thư mục dự án mà bạn đã tạo trong quá trình sử dụng.
- **Khoá API đã lưu**: Mở **Windows Credential Manager** (Quản lý thông tin xác thực Windows) → mục **Generic Credentials** → tìm và xoá các mục có tiền tố `studio.` nếu cần.
