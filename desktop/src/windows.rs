use crate::wallpaper;
use crate::Config;
use crate::TYPE_FULL;
use crate::TYPE_HALF;
use std::cell::RefCell;
use std::mem;
use std::path::Path;
use std::ptr::null_mut;
use std::sync::Mutex;
use winapi::shared::basetsd::UINT_PTR;
use winapi::shared::minwindef::{DWORD, HINSTANCE, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::ntdef::LPSTR;
use winapi::shared::windef::{HBRUSH, HICON, HMENU, HWND, POINT};
use winapi::um::shellapi::{
    ShellExecuteW, Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_NONE, NIM_ADD,
    NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
};
use winapi::um::wingdi::{GetStockObject, WHITE_BRUSH};
use winapi::um::winnt::LPCWSTR;
use winapi::um::winuser::*;

//https://blog.csdn.net/end_ing/article/details/19168679

pub static APP_NAME: &str = "himawari8壁纸";

const IDR_EXIT: usize = 10;
const IDR_HOME: usize = 20;
const IDR_TP_FULL: usize = 210;
const IDR_TP_HALF: usize = 211;
const IDR_FQ_10: usize = 110;
const IDR_FQ_20: usize = 111;
const IDR_FQ_30: usize = 112;
const IDR_FQ_60: usize = 113;

const MSG_ERROR: u32 = WM_USER + 100;
const MSG_OK: u32 = WM_USER + 101;
const MSG_PROGRESS: u32 = WM_USER + 102;

lazy_static! {
    static ref SCREEN_WIDTH: i32 = unsafe { GetSystemMetrics(SM_CXSCREEN) as i32 };
    static ref SCREEN_HEIGHT: i32 = unsafe { GetSystemMetrics(SM_CYSCREEN) as i32 };
    static ref WM_TASKBAR_CREATED: UINT =
        unsafe { RegisterWindowMessageW(convert("TaskbarCreated")) };
    static ref H_MENU: Mutex<isize> = Mutex::new(0);
    static ref TIMER_ID: Mutex<usize> = Mutex::new(0);
    static ref CONFIG: Mutex<Config> = Mutex::new(Config::default());
}

thread_local! {
    static NID:RefCell<NOTIFYICONDATAW> = RefCell::new(unsafe{std::mem::zeroed()});
}

//切换到整福图
fn switch_to_full() {
    let tid = thread_id::get();
    std::thread::spawn(move || {
        if wallpaper::set_full(
            *SCREEN_WIDTH,
            *SCREEN_HEIGHT,
            move |current: i32, total: i32| unsafe {
                PostThreadMessageW(tid as u32, MSG_PROGRESS, current as usize, total as isize);
            },
        )
        .is_err()
        {
            unsafe {
                PostThreadMessageW(tid as u32, MSG_ERROR, 0, 0);
            }
        } else {
            unsafe {
                PostThreadMessageW(tid as u32, MSG_OK, 0, 0);
            }
        }
    });
}

//切换到半幅图
fn switch_to_half() {
    let tid = thread_id::get();
    std::thread::spawn(move || {
        if wallpaper::set_half(
            *SCREEN_WIDTH,
            *SCREEN_HEIGHT,
            move |current: i32, total: i32| unsafe {
                PostThreadMessageW(tid as u32, MSG_PROGRESS, current as usize, total as isize);
            },
        )
        .is_err()
        {
            unsafe {
                PostThreadMessageW(tid as u32, MSG_ERROR, 0, 0);
            }
        } else {
            unsafe {
                PostThreadMessageW(tid as u32, MSG_OK, 0, 0);
            }
        }
    });
}

fn init_timer(h_wnd: HWND, min: i32) {
    //销毁时钟
    NID.with(|nid| {
        let nid = nid.borrow_mut();
        unsafe {
            KillTimer(nid.hWnd, *TIMER_ID.lock().unwrap());
        }
    });
    //启动定时器 10分钟一次, 30分钟一次, 60分钟一次
    unsafe extern "system" fn task(_: HWND, _: UINT, _: UINT_PTR, _: DWORD) {
        match CONFIG.lock().unwrap().show_type {
            TYPE_HALF => switch_to_half(),
            TYPE_FULL => switch_to_full(),
            _ => (),
        };
    }
    *TIMER_ID.lock().unwrap() = unsafe { SetTimer(h_wnd, 1, min as u32 * 60 * 1000, Some(task)) };
}

//窗口消息函数
pub unsafe extern "system" fn window_proc(
    h_wnd: HWND,
    u_msg: UINT,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let app_name = convert_u16(APP_NAME);
    match u_msg {
        WM_CREATE => {
            NID.with(|nid| {
                let mut nid = nid.borrow_mut();
                nid.cbSize = mem::size_of::<NOTIFYICONDATAW>() as u32;;
                nid.hWnd = h_wnd;
                nid.uID = 0;
                nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
                nid.uCallbackMessage = WM_USER;
                nid.hIcon = if Path::new("icon.ico").exists() {
                    LoadImageW(
                        0 as HINSTANCE,
                        convert("icon.ico"),
                        IMAGE_ICON,
                        0,
                        0,
                        LR_LOADFROMFILE,
                    ) as HICON
                } else {
                    LoadIconW(0 as HINSTANCE, IDI_APPLICATION)
                }; //图标
                nid.szTip
                    .get_mut(0..app_name.len())
                    .unwrap()
                    .copy_from_slice(&app_name);
                Shell_NotifyIconW(NIM_ADD, &mut *nid);
            });

            //二级菜单
            let fq_menu = CreatePopupMenu();
            AppendMenuW(fq_menu, MF_STRING, IDR_FQ_10, convert("10分钟"));
            AppendMenuW(fq_menu, MF_STRING, IDR_FQ_20, convert("20分钟"));
            AppendMenuW(fq_menu, MF_STRING, IDR_FQ_30, convert("30分钟"));
            AppendMenuW(fq_menu, MF_STRING, IDR_FQ_60, convert("1小时"));
            let ty_menu = CreatePopupMenu();
            AppendMenuW(ty_menu, MF_STRING, IDR_TP_FULL, convert("整幅图"));
            AppendMenuW(ty_menu, MF_STRING, IDR_TP_HALF, convert("半幅图"));

            //一级菜单
            let h_menu = CreatePopupMenu();
            AppendMenuW(h_menu, MF_POPUP, fq_menu as usize, convert("更新频率"));
            AppendMenuW(h_menu, MF_POPUP, ty_menu as usize, convert("展示方式"));
            AppendMenuW(h_menu, MF_STRING, IDR_HOME, convert("项目主页"));
            AppendMenuW(h_menu, MF_STRING, IDR_EXIT, convert("退出"));
            *H_MENU.lock().unwrap() = h_menu as isize;

            //启动时第一次下载
            //读取配置: 更新频率、展示方式
            let conf = CONFIG.lock().unwrap();
            if conf.show_type == TYPE_FULL {
                switch_to_full();
            } else {
                switch_to_half();
            }
            init_timer(h_wnd, conf.freq);

            //弹出气泡
            show_bubble("已启动");
        }
        WM_USER => {
            match l_param as u32 {
                WM_LBUTTONDBLCLK => {
                    SendMessageW(h_wnd, WM_CLOSE, w_param, l_param);
                }
                WM_RBUTTONDOWN | WM_LBUTTONDOWN => {
                    let mut pt: POINT = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt); //取鼠标坐标
                    SetForegroundWindow(h_wnd); //解决在菜单外单击左键菜单不消失的问题
                                                // EnableMenuItem(hmenu,IDR_PAUSE,MF_GRAYED);//让菜单中的某一项变灰
                    match TrackPopupMenu(
                        *H_MENU.lock().unwrap() as HMENU,
                        TPM_RETURNCMD,
                        pt.x,
                        pt.y,
                        0,
                        h_wnd,
                        null_mut(),
                    ) as usize
                    {
                        //显示菜单并获取选项ID
                        IDR_EXIT => {
                            SendMessageW(h_wnd, WM_CLOSE, w_param, l_param);
                        }
                        IDR_HOME => {
                            //打开github主页链接
                            ShellExecuteW(
                                h_wnd,
                                convert("open"),
                                convert("https://github.com/planet0104/himawari-8-wallpaper"),
                                null_mut(),
                                null_mut(),
                                SW_SHOWNORMAL,
                            );
                        }
                        IDR_TP_FULL => {
                            let mut conf = CONFIG.lock().unwrap();
                            if conf.show_type != TYPE_FULL {
                                conf.show_type = TYPE_FULL;
                                switch_to_full();
                                crate::write_config(&conf);
                            }
                        }
                        IDR_TP_HALF => {
                            let mut conf = CONFIG.lock().unwrap();
                            if conf.show_type != TYPE_HALF {
                                conf.show_type = TYPE_HALF;
                                switch_to_half();
                                crate::write_config(&conf);
                            }
                        }
                        IDR_FQ_10 => {
                            init_timer(h_wnd, 10);
                            let mut conf = CONFIG.lock().unwrap();
                            conf.freq = 10;
                            crate::write_config(&conf);
                        }
                        IDR_FQ_20 => {
                            init_timer(h_wnd, 20);
                            let mut conf = CONFIG.lock().unwrap();
                            conf.freq = 10;
                            crate::write_config(&conf);
                        }
                        IDR_FQ_30 => {
                            init_timer(h_wnd, 30);
                            let mut conf = CONFIG.lock().unwrap();
                            conf.freq = 30;
                            crate::write_config(&conf);
                        }
                        IDR_FQ_60 => {
                            init_timer(h_wnd, 60);
                            let mut conf = CONFIG.lock().unwrap();
                            conf.freq = 60;
                            crate::write_config(&conf);
                        }
                        0 => {
                            PostMessageW(h_wnd, WM_LBUTTONDOWN, 0, 0);
                        }
                        _ => {}
                    }
                }
                _ => (),
            }
        }
        WM_DESTROY => {
            println!("程序结束");
            NID.with(|nid| {
                let mut nid = nid.borrow_mut();
                //销毁时钟
                KillTimer(nid.hWnd, *TIMER_ID.lock().unwrap());
                //删除托盘
                Shell_NotifyIconW(NIM_DELETE, &mut *nid);
            });
            PostQuitMessage(0);
        }
        _ => {
            /*
             * 防止当Explorer.exe 崩溃以后，程序在系统系统托盘中的图标就消失
             *
             * 原理：Explorer.exe 重新载入后会重建系统任务栏。当系统任务栏建立的时候会向系统内所有
             * 注册接收TaskbarCreated 消息的顶级窗口发送一条消息，我们只需要捕捉这个消息，并重建系
             * 统托盘的图标即可。
             */
            if u_msg == *WM_TASKBAR_CREATED {
                SendMessageW(h_wnd, WM_CREATE, w_param, l_param);
            }
        }
    }
    DefWindowProcW(h_wnd, u_msg, w_param, l_param)
}

pub fn alert(title: &str, msg: &str) {
    unsafe {
        MessageBoxW(null_mut(), convert(msg), convert(title), MB_OK);
    }
}

#[allow(non_snake_case)]
pub fn win_main(
    hInstance: HINSTANCE,
    _hPrevInstance: HINSTANCE,
    _szCmdLine: LPSTR,
    iCmdShow: i32,
    conf: Config,
) -> i32 {
    let app_name = convert(APP_NAME);
    *CONFIG.lock().unwrap() = conf;

    let handle = unsafe { FindWindowW(null_mut(), app_name) };
    if !handle.is_null() {
        alert(APP_NAME, "程序已经运行");
        return 0;
    }

    let mut wndclass: WNDCLASSW = unsafe { std::mem::zeroed() };

    wndclass.style = CS_HREDRAW | CS_VREDRAW;
    wndclass.lpfnWndProc = Some(window_proc);
    wndclass.cbClsExtra = 0;
    wndclass.cbWndExtra = 0;
    wndclass.hInstance = hInstance;
    wndclass.hIcon = unsafe { LoadIconW(null_mut(), IDI_APPLICATION) };
    wndclass.hCursor = unsafe { LoadCursorW(null_mut(), IDC_ARROW) };
    wndclass.hbrBackground = unsafe { GetStockObject(WHITE_BRUSH as i32) as HBRUSH };
    wndclass.lpszMenuName = null_mut();
    wndclass.lpszClassName = app_name;

    if unsafe { RegisterClassW(&wndclass) == 0 } {
        alert(APP_NAME, "程序需要在Windows NT运行！");
        return 0;
    }

    // 此处使用WS_EX_TOOLWINDOW 属性来隐藏显示在任务栏上的窗口程序按钮
    let hwnd = unsafe {
        CreateWindowExW(
            WS_EX_TOOLWINDOW,
            app_name,
            app_name,
            WS_POPUP,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            null_mut(),
            null_mut(),
            hInstance,
            null_mut(),
        )
    };

    let mut msg: MSG = unsafe { std::mem::zeroed() };
    unsafe {
        ShowWindow(hwnd, iCmdShow);
        UpdateWindow(hwnd);
        while GetMessageW(&mut msg, null_mut(), 0, 0) != 0 {
            match msg.message {
                MSG_ERROR => {
                    show_bubble(&format!(
                        "图片下载出错，右键击菜单重试 {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                    ));
                }
                MSG_PROGRESS => {
                    show_tip(&format!(
                        "正在下载卫星图片({}/{})",
                        msg.wParam, msg.lParam
                    ));
                }
                MSG_OK => {
                    show_tip(&format!(
                        "壁纸下载完成 {}",
                        chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
                    ));
                }
                _ => {}
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    msg.wParam as i32
}

fn show_tip(tip: &str) {
    NID.with(|nid| {
        let mut nid = nid.borrow_mut();

        // _tcscpy(m_nid.szInfoTitle,"提醒你");
        // _tcscpy(m_nid.szInfo,"内容改变");
        // m_nid.uTimeout=1000;
        // m_nid.uVersion=NOTIFYICON_VERSION;
        // Shell_NotifyIcon(NIM_MODIFY,&m_nid);

        nid.uFlags = NIF_TIP;
        let tip = convert_u16(tip);
        nid.szTip
            .get_mut(0..tip.len())
            .unwrap()
            .copy_from_slice(&tip);
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &mut *nid);
        }
    });
}

fn show_bubble(info: &str) {
    NID.with(|nid| {
        let mut nid = nid.borrow_mut();
        let title = convert_u16(APP_NAME);
        nid.szInfoTitle
            .get_mut(0..title.len())
            .unwrap()
            .copy_from_slice(&title);
        let info = convert_u16(info);
        nid.szInfo
            .get_mut(0..info.len())
            .unwrap()
            .copy_from_slice(&info);
        nid.uFlags = NIF_INFO;
        nid.dwInfoFlags = NIIF_NONE;
        unsafe {
            Shell_NotifyIconW(NIM_MODIFY, &mut *nid);
        }
    });
}

pub fn convert(s: &str) -> LPCWSTR {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v.as_ptr()
}

/** 字符串转换成双字 0结尾的数组 */
pub fn convert_u16(s: &str) -> Vec<u16> {
    let mut v: Vec<u16> = s.encode_utf16().collect();
    v.push(0);
    v
}