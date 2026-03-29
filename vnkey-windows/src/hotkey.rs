//! Cài đặt và gán phím tắt (Win32 native).

use crate::ui;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{LazyLock, Mutex};

use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::UI::WindowsAndMessaging::*;

// ── Cài đặt phím tắt (toàn cục) ────────────────────────────────────────────

#[derive(Debug)]
pub struct HotkeySettings {
    pub toggle_vk: u32,
    pub toggle_mods: u32,
    pub conv_vk: u32,
    pub conv_mods: u32,
}

impl Default for HotkeySettings {
    fn default() -> Self {
        Self { toggle_vk: 0, toggle_mods: 0, conv_vk: 0, conv_mods: 0 }
    }
}

pub static HOTKEY_SETTINGS: LazyLock<Mutex<HotkeySettings>> =
    LazyLock::new(|| Mutex::new(HotkeySettings::default()));

pub const HOTKEY_ID_TOGGLE: i32 = 9000;
pub const HOTKEY_ID_CONVERT: i32 = 9001;

// ── Trợ giúp hiển thị ──────────────────────────────────────────────────────

pub fn hotkey_display_text(vk: u32, mods: u32) -> String {
    if vk == 0 && mods == 0 { return "(chưa đặt)".into(); }
    let mut s = String::new();
    if mods & 2 != 0 { s.push_str("Ctrl+"); }
    if mods & 1 != 0 { s.push_str("Alt+"); }
    if mods & 4 != 0 { s.push_str("Shift+"); }
    if vk == 0 {
        if s.ends_with('+') { s.truncate(s.len() - 1); }
        return s;
    }
    let key_name = match vk {
        0x41..=0x5A => format!("{}", (vk as u8) as char),
        0x30..=0x39 => format!("{}", (vk as u8 - 0x30)),
        0x70..=0x87 => format!("F{}", vk - 0x6F),
        0x20 => "Space".into(),
        0x1B => "Esc".into(),
        0x0D => "Enter".into(),
        0x09 => "Tab".into(),
        _ => format!("0x{:02X}", vk),
    };
    s.push_str(&key_name);
    s
}

fn toggle_display(hk: &HotkeySettings) -> String {
    if hk.toggle_vk == 0 && hk.toggle_mods == 0 {
        "Ctrl+Shift (mặc định)".into()
    } else {
        hotkey_display_text(hk.toggle_vk, hk.toggle_mods)
    }
}

// ── Đăng ký/hủy đăng ký ─────────────────────────────────────────────────

pub fn register_hotkeys(hwnd: HWND) {
    if let Ok(hk) = HOTKEY_SETTINGS.lock() {
        unsafe {
            // Phím tắt toggle Việt/Anh: KHÔNG dùng RegisterHotKey vì
            // WM_HOTKEY kích hoạt hidden window → mất focus ứng dụng đang soạn thảo.
            // Thay vào đó, xử lý trực tiếp trong LL keyboard hook (hook.rs).

            // Chỉ RegisterHotKey cho phím chuyển mã clipboard (không gây vấn đề focus
            // vì clipboard convert không cần giữ focus ở ứng dụng nào).
            if hk.conv_vk != 0 {
                if let Err(e) = RegisterHotKey(hwnd, HOTKEY_ID_CONVERT,
                    HOT_KEY_MODIFIERS(hk.conv_mods), hk.conv_vk) {
                    eprintln!("[hotkey] RegisterHotKey conv FAILED: {e} (vk={:#x}, mods={:#x}, hwnd={:?})",
                        hk.conv_vk, hk.conv_mods, hwnd);
                }
            }
        }
    }
}

pub fn unregister_hotkeys(hwnd: HWND) {
    unsafe {
        // Toggle hotkey không còn dùng RegisterHotKey (xử lý trong LL hook).
        // Vẫn gọi Unregister phòng trường hợp phiên bản cũ đã đăng ký.
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_TOGGLE);
        let _ = UnregisterHotKey(hwnd, HOTKEY_ID_CONVERT);
    }
}

fn reregister_hotkeys() {
    // RegisterHotKey phải được gọi từ thread sở hữu cửa sổ.
    // Hộp thoại phím tắt chạy trên thread riêng, nên ta gửi tin nhắn
    // tới cửa sổ chính để nó đăng ký lại trên thread của nó.
    unsafe {
        match FindWindowW(w!("VnKeyHiddenWindow"), w!("VnKey")) {
            Ok(mw) => {
                let _ = PostMessageW(mw, crate::WM_VNKEY_REREGISTER_HOTKEYS,
                    WPARAM(0), LPARAM(0));
            }
            Err(e) => {
                eprintln!("[hotkey] FindWindowW FAILED: {e}");
            }
        }
    }
}

pub fn is_toggle_builtin() -> bool {
    match HOTKEY_SETTINGS.lock() {
        Ok(hk) => hk.toggle_vk == 0 && hk.toggle_mods == 0,
        Err(_) => true,
    }
}

// ── Mã điều khiển ───────────────────────────────────────────────────────

const ID_LBL_TOGGLE: u16   = 300;
const ID_LBL_CONV: u16     = 301;
const ID_BTN_SET_TOG: u16  = 302;
const ID_BTN_CLR_TOG: u16  = 303;
const ID_BTN_SET_CONV: u16 = 304;
const ID_BTN_CLR_CONV: u16 = 305;
const ID_BTN_CLOSE: u16    = 306;

/// Phím tắt nào đang được bắt (None = không bắt)
static CAPTURE_TARGET: Mutex<Option<CaptureTarget>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
enum CaptureTarget {
    Toggle,
    Conv,
}

// ── Cửa sổ giao diện ────────────────────────────────────────────────────────

static HK_OPEN: AtomicBool = AtomicBool::new(false);

pub fn open_hotkey_window() {
    if HK_OPEN.swap(true, Ordering::SeqCst) { return; }
    std::thread::spawn(|| {
        run_hotkey();
        HK_OPEN.store(false, Ordering::Relaxed);
    });
}

fn run_hotkey() {
    ui::init_common_controls();

    let hwnd = ui::create_dialog_window(
        w!("VnKeyHotkey"),
        &format!("Gán phím – VnKey {}", crate::gui::VERSION),
        420, 178,
        Some(hotkey_wnd_proc),
    );
    if hwnd.0.is_null() { return; }
    create_controls(hwnd);
    ui::show_and_focus(hwnd);

    // Vòng lặp tin nhắn tùy chỉnh: bỏ qua IsDialogMessageW khi bắt phím,
    // nếu không thì tin nhắn bàn phím bị dialog navigation nuốt mất.
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            let capturing = CAPTURE_TARGET.lock().map_or(false, |t| t.is_some());
            if capturing {
                // Chuyển hướng tin nhắn bàn phím về cửa sổ cha để
                // wndproc nhận được (bình thường chúng đi tới nút đang focus)
                if matches!(msg.message, WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP) {
                    msg.hwnd = hwnd;
                }
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            } else if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

fn create_controls(hwnd: HWND) {
    let font = ui::create_ui_font();
    let font_bold = ui::create_ui_font_bold();

    let (toggle_text, conv_text) = {
        let hk = HOTKEY_SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
        (toggle_display(&hk), hotkey_display_text(hk.conv_vk, hk.conv_mods))
    };

    // Nhóm
    ui::create_groupbox("Phím tắt", 10, 5, 400, 120, hwnd, 350, font);

    // Hàng 1: Chuyển đổi
    ui::create_label("Chuyển Anh/Việt:", 22, 28, 120, 18, hwnd, 351, font);
    let lbl = ui::create_label(&toggle_text, 145, 28, 140, 18, hwnd, ID_LBL_TOGGLE, font_bold);
    // Đổi màu nhãn
    let _ = lbl;
    ui::create_button("Đặt", 290, 25, 50, 24, hwnd, ID_BTN_SET_TOG, font);
    ui::create_button("Xóa", 345, 25, 50, 24, hwnd, ID_BTN_CLR_TOG, font);

    // Hàng 2: Chuyển mã clipboard
    ui::create_label("Chuyển mã clipboard:", 22, 60, 120, 18, hwnd, 352, font);
    ui::create_label(&conv_text, 145, 60, 140, 18, hwnd, ID_LBL_CONV, font_bold);
    ui::create_button("Đặt", 290, 57, 50, 24, hwnd, ID_BTN_SET_CONV, font);
    ui::create_button("Xóa", 345, 57, 50, 24, hwnd, ID_BTN_CLR_CONV, font);

    // Gợi ý bắt phím (ẩn cho đến khi đang bắt)
    ui::create_label("", 22, 90, 370, 18, hwnd, 360, font);

    // Nút đóng
    ui::create_button("Đóng", 155, 135, 110, 32, hwnd, ID_BTN_CLOSE, font);
}

unsafe extern "system" fn hotkey_wnd_proc(
    hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as u16;
            handle_command(hwnd, id);
            LRESULT(0)
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            if let Ok(target) = CAPTURE_TARGET.lock() {
                if target.is_some() {
                    let vk = wparam.0 as u32;
                    // Bỏ qua các phím chỉ là modifier
                    if matches!(vk, 0x10 | 0x11 | 0x12 | 0xA0..=0xA5) {
                        return LRESULT(0);
                    }
                    let mut mods = 0u32;
                    if GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000 != 0 { mods |= 2; }
                    if GetAsyncKeyState(VK_MENU.0 as i32) as u16 & 0x8000 != 0 { mods |= 1; }
                    if GetAsyncKeyState(VK_SHIFT.0 as i32) as u16 & 0x8000 != 0 { mods |= 4; }

                    let target_copy = *target;
                    drop(target);

                    apply_capture(hwnd, target_copy.unwrap(), vk, mods);
                    return LRESULT(0);
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN => {
            let hdc = HDC(wparam.0 as _);
            SetBkColor(hdc, ui::BG_COLOR);
            SetTextColor(hdc, COLORREF(0x001A1A1A));
            LRESULT(ui::BG_BRUSH().0 as _)
        }
        WM_CLOSE => { DestroyWindow(hwnd).ok(); LRESULT(0) }
        WM_DESTROY => {
            // Xóa trạng thái bắt phím
            if let Ok(mut t) = CAPTURE_TARGET.lock() { *t = None; }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

fn handle_command(hwnd: HWND, id: u16) {
    match id {
        ID_BTN_SET_TOG => start_capture(hwnd, CaptureTarget::Toggle),
        ID_BTN_SET_CONV => start_capture(hwnd, CaptureTarget::Conv),
        ID_BTN_CLR_TOG => {
            if let Ok(mut hk) = HOTKEY_SETTINGS.lock() {
                hk.toggle_vk = 0;
                hk.toggle_mods = 0;
            }
            reregister_hotkeys();
            crate::config::save();
            let lbl = unsafe { GetDlgItem(hwnd, ID_LBL_TOGGLE as i32) }.unwrap_or_default();
            ui::set_window_text(lbl, "Ctrl+Shift (mặc định)");
        }
        ID_BTN_CLR_CONV => {
            if let Ok(mut hk) = HOTKEY_SETTINGS.lock() {
                hk.conv_vk = 0;
                hk.conv_mods = 0;
            }
            if let Ok(mut cs) = crate::converter::CONV_SETTINGS.lock() {
                cs.hotkey_vk = 0;
                cs.hotkey_modifiers = 0;
            }
            reregister_hotkeys();
            crate::config::save();
            let lbl = unsafe { GetDlgItem(hwnd, ID_LBL_CONV as i32) }.unwrap_or_default();
            ui::set_window_text(lbl, "(chưa đặt)");
        }
        ID_BTN_CLOSE => { unsafe { DestroyWindow(hwnd).ok(); } }
        _ => {}
    }
}

fn start_capture(hwnd: HWND, target: CaptureTarget) {
    if let Ok(mut t) = CAPTURE_TARGET.lock() {
        *t = Some(target);
    }
    let lbl_id = match target {
        CaptureTarget::Toggle => ID_LBL_TOGGLE,
        CaptureTarget::Conv => ID_LBL_CONV,
    };
    let lbl = unsafe { GetDlgItem(hwnd, lbl_id as i32) }.unwrap_or_default();
    ui::set_window_text(lbl, "Nhấn phím...");
    // Hiển thị gợi ý
    let hint = unsafe { GetDlgItem(hwnd, 360) }.unwrap_or_default();
    ui::set_window_text(hint, "Nhấn tổ hợp phím rồi thả. Esc để hủy.");
}

fn apply_capture(hwnd: HWND, target: CaptureTarget, vk: u32, mods: u32) {
    // Esc hủy bắt phím
    if vk == 0x1B {
        if let Ok(mut t) = CAPTURE_TARGET.lock() { *t = None; }
        // Khôi phục hiển thị ban đầu
        let (toggle_text, conv_text) = {
            let hk = HOTKEY_SETTINGS.lock().unwrap_or_else(|e| e.into_inner());
            (toggle_display(&hk), hotkey_display_text(hk.conv_vk, hk.conv_mods))
        };
        let lbl_tog = unsafe { GetDlgItem(hwnd, ID_LBL_TOGGLE as i32) }.unwrap_or_default();
        let lbl_conv = unsafe { GetDlgItem(hwnd, ID_LBL_CONV as i32) }.unwrap_or_default();
        ui::set_window_text(lbl_tog, &toggle_text);
        ui::set_window_text(lbl_conv, &conv_text);
        let hint = unsafe { GetDlgItem(hwnd, 360) }.unwrap_or_default();
        ui::set_window_text(hint, "");
        return;
    }

    let display = hotkey_display_text(vk, mods);

    match target {
        CaptureTarget::Toggle => {
            if let Ok(mut hk) = HOTKEY_SETTINGS.lock() {
                hk.toggle_vk = vk;
                hk.toggle_mods = mods;
            }
            let lbl = unsafe { GetDlgItem(hwnd, ID_LBL_TOGGLE as i32) }.unwrap_or_default();
            ui::set_window_text(lbl, &display);
        }
        CaptureTarget::Conv => {
            if let Ok(mut hk) = HOTKEY_SETTINGS.lock() {
                hk.conv_vk = vk;
                hk.conv_mods = mods;
            }
            if let Ok(mut cs) = crate::converter::CONV_SETTINGS.lock() {
                cs.hotkey_vk = vk;
                cs.hotkey_modifiers = mods;
            }
            let lbl = unsafe { GetDlgItem(hwnd, ID_LBL_CONV as i32) }.unwrap_or_default();
            ui::set_window_text(lbl, &display);
        }
    }

    reregister_hotkeys();
    crate::config::save();

    // Kết thúc bắt phím
    if let Ok(mut t) = CAPTURE_TARGET.lock() { *t = None; }
    let hint = unsafe { GetDlgItem(hwnd, 360) }.unwrap_or_default();
    ui::set_window_text(hint, "");
}
