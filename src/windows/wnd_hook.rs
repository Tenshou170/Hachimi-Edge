use std::{os::raw::c_uint, sync::{atomic::{self, AtomicIsize, AtomicUsize}, Arc}};

use windows::{core::{w, BOOL}, Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::{GetCurrentProcessId, GetCurrentThreadId},
    UI::{
        Input::Ime::ISC_SHOWUICOMPOSITIONWINDOW,
        WindowsAndMessaging::{
            CallNextHookEx, DefWindowProcW, EnumWindows, FindWindowW, GetClassNameW, GetWindowLongPtrW, GetWindowThreadProcessId, SetWindowsHookExW, UnhookWindowsHookEx,
            GWLP_WNDPROC, HCBT_MINMAX, HHOOK, SW_RESTORE, WH_CBT, WM_CLOSE, WM_KEYDOWN, WM_SYSKEYDOWN, WNDPROC,
            WM_IME_SETCONTEXT, WM_IME_NOTIFY, WM_ACTIVATE, WA_INACTIVE
        },
    }
}};

use crate::{core::{game::Region, Gui, Hachimi}, il2cpp::{hook::UnityEngine_CoreModule, symbols::Thread}, windows::utils};
use rust_i18n::t;

use super::{gui_impl::input, discord, smtc, taskbar};

static TARGET_HWND: AtomicIsize = AtomicIsize::new(0);
pub fn get_target_hwnd() -> HWND {
    HWND(TARGET_HWND.load(atomic::Ordering::Relaxed) as *mut _)
}

pub fn set_target_hwnd(hwnd: HWND) {
    TARGET_HWND.store(hwnd.0 as isize, atomic::Ordering::Relaxed);
}

fn find_window_by_class_in_current_process() -> HWND {
    struct WindowSearchState {
        process_id: u32,
        hwnd: HWND,
    }

    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let state = &mut *(lparam.0 as *mut WindowSearchState);
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid != state.process_id {
            return BOOL(1);
        }

        let mut class_name = [0u16; 32];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len <= 0 {
            return BOOL(1);
        }

        let class = String::from_utf16_lossy(&class_name[..len as usize]);
        if class == "UnityWndClass" {
            state.hwnd = hwnd;
            return BOOL(0);
        }

        BOOL(1)
    }

    let mut state = WindowSearchState {
        process_id: unsafe { GetCurrentProcessId() },
        hwnd: HWND(std::ptr::null_mut()),
    };

    unsafe {
        let _ = EnumWindows(Some(enum_windows_proc), LPARAM(&mut state as *mut _ as isize));
    }

    if !state.hwnd.0.is_null() {
        info!("find_window_by_class_in_current_process: found UnityWndClass hwnd={:?}", state.hwnd);
    } else {
        info!("find_window_by_class_in_current_process: no UnityWndClass window found in current process");
    }

    state.hwnd
}

static MENU_KEY_CAPTURE: atomic::AtomicBool = atomic::AtomicBool::new(false);
pub fn start_menu_key_capture() {
    MENU_KEY_CAPTURE.store(true, atomic::Ordering::Relaxed);
}

// thread and writes from init() are race-free.
static WNDPROC_ORIG: AtomicIsize = AtomicIsize::new(0);
static WNDPROC_RECALL: AtomicUsize = AtomicUsize::new(0);
extern "system" fn wnd_proc(hwnd: HWND, umsg: c_uint, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    let orig_raw = WNDPROC_ORIG.load(atomic::Ordering::Relaxed);
    let Some(orig_fn) = (unsafe { std::mem::transmute::<isize, WNDPROC>(orig_raw) }) else {
        return unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) };
    };

    match umsg {
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let current_key = wparam.0 as u16;

            if current_key == 0x4B { // Virtual keycode for "K", see the get_key method on gui_impl/input.rs
                let hotkey_vk = Hachimi::instance().config.load().windows.hide_ingame_ui_hotkey_bind;

                if unsafe { windows::Win32::UI::Input::KeyboardAndMouse::GetKeyState(hotkey_vk as i32) < 0 } {
                    if let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap_or_else(|e| e.into_inner())) {
                        gui.set_consuming_input(false);
                    }
                    return LRESULT(0); 
                }
            }

            if MENU_KEY_CAPTURE.load(atomic::Ordering::Relaxed) {
                MENU_KEY_CAPTURE.store(false, atomic::Ordering::Relaxed);
                let hachimi = Hachimi::instance();
                let mut new_config = hachimi.config.load().as_ref().clone();
                new_config.windows.menu_open_key = current_key;
                let _ = hachimi.save_config(&new_config);
                hachimi.config.store(Arc::new(new_config));
                let key_label = crate::windows::utils::vk_to_display_label(Hachimi::instance().config.load().windows.menu_open_key);
                let msg = t!("notification.menu_open_key_set", key = key_label);
                std::thread::spawn(move || {
                    if let Some(gui) = Gui::instance() {
                        gui.lock().unwrap_or_else(|e| e.into_inner()).show_notification(&msg);
                    }
                });
                return LRESULT(0);
            }
            // Generic keybind capture — used by SetKeybindWindow for rebinding
            // keys like hide_ingame_ui_hotkey_bind.
            if crate::core::gui::is_keybind_capture_active() {
                let display = crate::windows::utils::vk_to_display_label(current_key);
                crate::core::gui::report_keybind_capture(current_key, display);
                return LRESULT(0);
            }
            if current_key == Hachimi::instance().config.load().windows.menu_open_key {
                let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap_or_else(|e| e.into_inner())) else {
                    return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
                };
                gui.toggle_menu();
                return LRESULT(0);
            } else if current_key == Hachimi::instance().config.load().windows.hide_ingame_ui_hotkey_bind && Hachimi::instance().config.load().hide_ingame_ui_hotkey {
                Thread::main_thread().schedule(Gui::toggle_game_ui);
            }
        },
        WM_ACTIVATE => {
            let res = unsafe { orig_fn(hwnd, umsg, wparam, lparam) };

            if (wparam.0 & 0xFFFF) != WA_INACTIVE as usize {
                std::thread::spawn(move || {
                    if let Some(gui) = Gui::instance().map(|m| m.lock().unwrap_or_else(|e| e.into_inner())) {
                        if gui.context.wants_keyboard_input() {
                            Thread::main_thread().schedule(|| {
                                crate::il2cpp::hook::UnityEngine_InputLegacyModule::Input::set_imeCompositionMode(1);
                            });
                        }
                    }
                });
            }
            return res;
        },
        WM_CLOSE => {
            if let Some(hook) = Hachimi::instance().interceptor.unhook(wnd_proc as *const () as _) {
                WNDPROC_RECALL.store(hook.orig_addr, atomic::Ordering::Release);
                let threads = Thread::attached_threads();
                if let Some(main) = threads.first() {
                    main.schedule(|| {
                        let recall_addr = WNDPROC_RECALL.load(atomic::Ordering::Acquire);
                        if recall_addr != 0 {
                            if let Some(orig_fn) = unsafe { std::mem::transmute::<usize, WNDPROC>(recall_addr) } {
                                unsafe { orig_fn(get_target_hwnd(), WM_CLOSE, WPARAM(0), LPARAM(0)); }
                            }
                        }
                    });
                } else {
                    warn!("[wnd_hook] no attached threads on WM_CLOSE, cannot dispatch");
                }
            }
            return LRESULT(0);
        },
        _ => ()
    }

    // Only capture input if gui needs it
    if !Gui::is_consuming_input_atomic() {
        return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
    }

    if umsg == WM_IME_SETCONTEXT {
        let new_lparam = lparam.0 & !(ISC_SHOWUICOMPOSITIONWINDOW as isize);
        if Gui::is_consuming_input_atomic() {
            return unsafe { DefWindowProcW(hwnd, umsg, wparam, LPARAM(new_lparam)) };
        }
        return unsafe { orig_fn(hwnd, umsg, wparam, LPARAM(new_lparam)) };
    }

    if umsg == WM_IME_NOTIFY {
        if Gui::is_consuming_input_atomic() {
            return unsafe { DefWindowProcW(hwnd, umsg, wparam, lparam) };
        }
    }

    // Extract the IME data BEFORE spanning the thread
    let (is_ime, ime_commit, ime_preedit) = input::process_ime_sync(hwnd, umsg, lparam.0);

    // Check if the input processor handles this message (Skip check if it is an IME msg)
    if !input::is_handled_msg(umsg) && !is_ime {
        return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
    }

    // A deadlock would *sometimes* consistently occur if this was done on the current thread
    // (when moving the window, etc.)
    // I assume that SwapChain::Present and WndProc are running on the same thread
    std::thread::spawn(move || {
        let Some(mut gui) = Gui::instance().map(|m| m.lock().unwrap_or_else(|e| e.into_inner())) else {
            return;
        };

        // Inject IME strings directly into egui
        if let Some(s) = ime_commit {
            gui.input.events.push(egui::Event::Ime(egui::ImeEvent::Commit(s)));
        }
        if let Some(s) = ime_preedit {
            gui.input.events.push(egui::Event::Ime(egui::ImeEvent::Preedit(s)));
        }

        // Process standard Key/Mouse inputs ONLY if it wasn't an IME message
        if !is_ime {
            let zoom_factor = gui.context.zoom_factor();
            input::process(&mut gui.input, zoom_factor, umsg, wparam.0, lparam.0);
        }
    });

    if is_ime {
        return LRESULT(0);
    }

    if !Gui::wants_input_atomic() {
        return unsafe { orig_fn(hwnd, umsg, wparam, lparam) };
    }

    LRESULT(0)
}

static HCBTHOOK: AtomicIsize = AtomicIsize::new(0);
extern "system" fn cbt_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if ncode == HCBT_MINMAX as i32 &&
        lparam.0 as i32 != SW_RESTORE.0 &&
        Hachimi::instance().config.load().windows.block_minimize_in_full_screen &&
        UnityEngine_CoreModule::Screen::get_fullScreen()
    {
        return LRESULT(1);
    }

    let raw = HCBTHOOK.load(atomic::Ordering::Relaxed);
    let hook = if raw != 0 { Some(HHOOK(raw as *mut _)) } else { None };
    unsafe { CallNextHookEx(hook, ncode, wparam, lparam) }
}

pub fn init() {
    unsafe {
        let hachimi = Hachimi::instance();
        let game = &hachimi.game;

        let window_name = if game.region == Region::Japan && game.is_steam_release {
            // lmao
            w!("UmamusumePrettyDerby_Jpn")
        }
        else {
            // global technically has "Umamusume" as its title but this api
            // is case insensitive so it works. why am i surprised
            w!("umamusume")
        };
        let mut hwnd = FindWindowW(w!("UnityWndClass"), window_name).unwrap_or_default();
        if !hwnd.0.is_null() {
            info!("Game window found by title: {:?}", hwnd);
        } else {
            // Fallback to any Unity window if the title isn't available yet or has changed.
            hwnd = FindWindowW(w!("UnityWndClass"), None).unwrap_or_default();
            if !hwnd.0.is_null() {
                info!("Game window found by class fallback without title: {:?}", hwnd);
            }
        }

        if hwnd.0.is_null() {
            hwnd = find_window_by_class_in_current_process();
            if !hwnd.0.is_null() {
                info!("Game window found by process-owned class enumeration: {:?}", hwnd);
            }
        }

        if hwnd.0.is_null() {
            error!("Failed to find game window");
            return;
        }

        set_target_hwnd(hwnd);

        let title = hachimi.config.load().custom_title_name.clone();
        if let Some(t) = title {
            use windows::Win32::UI::WindowsAndMessaging::SetWindowTextW;
            use windows::core::HSTRING;
            let _ = SetWindowTextW(hwnd, &HSTRING::from(t));
        }

        if crate::windows::capabilities::supports_taskbar_progress() {
            taskbar::init(hwnd);
        }

        info!("Hooking WndProc");
        let wnd_proc_addr = GetWindowLongPtrW(hwnd, GWLP_WNDPROC);
        match hachimi.interceptor.hook(wnd_proc_addr as _, wnd_proc as *const () as _) {
            Ok(trampoline_addr) => WNDPROC_ORIG.store(trampoline_addr as isize, atomic::Ordering::Release),
            Err(e) => error!("Failed to hook WndProc: {}", e)
        }

        info!("Adding CBT hook");
        if let Ok(hhook) = SetWindowsHookExW(WH_CBT, Some(cbt_proc), None, GetCurrentThreadId()) {
            HCBTHOOK.store(hhook.0 as isize, atomic::Ordering::Release);
        }

        // Apply always on top
        if hachimi.window_always_on_top.load(atomic::Ordering::Relaxed) {
            _ = utils::set_window_topmost(hwnd, true);
        }

        if hachimi.discord_rpc.load(atomic::Ordering::Relaxed) {
            if let Err(e) = discord::start_rpc() {
                 error!("{}", e);
             }
        }

        if crate::windows::capabilities::supports_smtc() {
            smtc::init(hwnd);
        }
    }
}

pub fn uninit() {
    let raw = HCBTHOOK.swap(0, atomic::Ordering::AcqRel);
    if raw != 0 {
        info!("Removing CBT hook");
        if let Err(e) = unsafe { UnhookWindowsHookEx(HHOOK(raw as *mut _)) } {
            error!("Failed to remove CBT hook: {}", e);
        }
    }
    if let Err(e) = discord::stop_rpc() {
        error!("{}", e);
    }
}
