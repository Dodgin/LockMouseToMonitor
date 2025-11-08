use std::{ptr, thread, time::Duration};
use winapi::shared::minwindef::BOOL;
use winapi::shared::windef::{POINT, RECT, HMONITOR, HDC};
use winapi::um::winuser::{
    GetCursorPos, ClipCursor, MonitorFromPoint, GetMonitorInfoW, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, GetAsyncKeyState, VK_CONTROL, VK_F11, VK_LMENU, EnumDisplayMonitors,
};

#[derive(Clone)]
struct MonitorInfo {
    handle: HMONITOR,
    rect: RECT,
}

unsafe extern "system" fn monitor_enum_proc(
    hmonitor: HMONITOR,
    _: HDC,
    _: *mut RECT,
    data: isize,
) -> BOOL {
    let monitors = &mut *(data as *mut Vec<MonitorInfo>);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    
    if GetMonitorInfoW(hmonitor, &mut mi) != 0 {
        monitors.push(MonitorInfo {
            handle: hmonitor,
            rect: mi.rcMonitor,
        });
    }
    1 // continue enumeration
}

fn get_all_monitors() -> Vec<MonitorInfo> {
    let mut monitors = Vec::new();
    let monitors_ptr = &mut monitors as *mut Vec<MonitorInfo>;
    unsafe {
        EnumDisplayMonitors(
            ptr::null_mut(),
            ptr::null(),
            Some(monitor_enum_proc),
            monitors_ptr as isize,
        );
    }
    // Sort monitors by their left coordinate for consistent ordering
    monitors.sort_by_key(|m: &MonitorInfo| m.rect.left);
    monitors
}

fn get_current_monitor_index(monitors: &[MonitorInfo]) -> Option<usize> {
    unsafe {
        let mut pt: POINT = std::mem::zeroed();
        if GetCursorPos(&mut pt) == 0 {
            return None;
        }
        // Find which monitor contains the cursor
        monitors.iter().position(|m| point_in_rect(&pt, &m.rect))
    }
}

fn get_monitor_rect_for_point(x: i32, y: i32) -> Option<RECT> {
    let pt = POINT { x, y };
    let hmon = unsafe { MonitorFromPoint(pt, MONITOR_DEFAULTTONEAREST) };
    if hmon.is_null() {
        return None;
    }
    let mut mi: MONITORINFO = unsafe { std::mem::zeroed() };
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let ok = unsafe { GetMonitorInfoW(hmon, &mut mi as *mut MONITORINFO) };
    if ok == 0 {
        return None;
    }
    Some(mi.rcMonitor)
}

fn point_in_rect(pt: &POINT, rc: &RECT) -> bool {
    pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom
}

fn at_rect_edge(pt: &POINT, rc: &RECT) -> bool {
    // consider 1-pixel margin as "edge"
    pt.x <= rc.left + 1 || pt.x >= rc.right - 1 || pt.y <= rc.top + 1 || pt.y >= rc.bottom - 1
}

fn main() {
    println!("lockmousetomonitor - locks cursor to selected monitor");
    println!("Controls:");
    println!("- Press Ctrl to temporarily release lock when cursor reaches monitor edge");
    println!("- Press F11 to change which monitor is locked (while cursor is on the desired monitor)");
    println!("\nAvailable monitors:");

    let monitors = get_all_monitors();
    if monitors.is_empty() {
        println!("No monitors found!");
        return;
    }

    // Find which monitor currently contains the cursor
    let current_monitor_idx = get_current_monitor_index(&monitors);
    
    for (i, monitor) in monitors.iter().enumerate() {
        let current_marker = if Some(i) == current_monitor_idx { " (current)" } else { "" };
        println!("{}. Monitor {}: {}x{} at ({}, {}) to ({}, {}){}", 
            i + 1,
            i + 1,
            monitor.rect.right - monitor.rect.left,
            monitor.rect.bottom - monitor.rect.top,
            monitor.rect.left, monitor.rect.top,
            monitor.rect.right, monitor.rect.bottom,
            current_marker
        );
    }

    println!("\nEnter monitor number to lock to (1-{}), or press Enter for current monitor:", monitors.len());
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();

    let initial_rect = if input.is_empty() {
        // Use current monitor if we found one
        current_monitor_idx.map(|idx| monitors[idx].rect)
    } else {
        // Parse user selection
        match input.parse::<usize>() {
            Ok(n) if n > 0 && n <= monitors.len() => {
                Some(monitors[n - 1].rect)
            }
            _ => {
                println!("Invalid monitor number!");
                return;
            }
        }
    };

    let mut prev_ctrl = false;
    let mut release_on_exit = false;
    let mut clipped = false;
    let mut current_rect: Option<RECT> = None;

    // Initial lock using selected monitor
    if let Some(rc) = initial_rect {
        unsafe {
            let rc_ptr: *const RECT = &rc as *const RECT;
            if ClipCursor(rc_ptr) != 0 {
                clipped = true;
                current_rect = Some(rc);
                println!("Locked to monitor rect: left={} top={} right={} bottom={}", 
                    rc.left, rc.top, rc.right, rc.bottom);
            }
        }
    } else {
        println!("Failed to get monitor rectangle!");
        return;
    }

    loop {
        thread::sleep(Duration::from_millis(16)); // ~60Hz check rate

        // poll cursor and keyboard state
        let mut pt: POINT = unsafe { std::mem::zeroed() };
        let got = unsafe { GetCursorPos(&mut pt) };
        if got == 0 {
            continue;
        }

        let ctrl_pressed = unsafe { (GetAsyncKeyState(VK_CONTROL) as i16) < 0 };
        let lalt_pressed = unsafe { (GetAsyncKeyState(VK_LMENU) as i16) < 0 };
        let f11_pressed = unsafe { (GetAsyncKeyState(VK_F11) as i16) < 0 };

        let release_key_pressed = ctrl_pressed || lalt_pressed;

        // Always reapply clipping if we're supposed to be clipped
        // This ensures it stays active even after alt-tab
        if clipped && !release_on_exit {
            if let Some(rc) = &current_rect {
                unsafe { ClipCursor(rc) };
            }
        }

        if release_key_pressed && !prev_ctrl {
            // Release key-down event
            release_on_exit = true;
            println!("Ctrl/Alt pressed: will release the clip the next time the cursor hits the monitor edge");
        }
        prev_ctrl = release_key_pressed;

        // Handle monitor edge detection and release
        if let Some(rc) = &current_rect {
            if clipped && release_on_exit && at_rect_edge(&pt, rc) {
                unsafe { ClipCursor(ptr::null()) };
                clipped = false;
                println!("Released clip – you can move to other monitors now");
            } else if !clipped && point_in_rect(&pt, rc) {
                // Re-lock when returning to monitor
                unsafe { ClipCursor(rc) };
                clipped = true;
                release_on_exit = false;
                println!("Cursor returned to monitor; re-locked");
            }
        }

        // Handle F11 monitor switching
        if f11_pressed {
            if let Some(new_rc) = get_monitor_rect_for_point(pt.x, pt.y) {
                // Check if this is actually a different monitor
                if let Some(cur) = &current_rect {
                    if new_rc.left != cur.left || new_rc.top != cur.top || 
                       new_rc.right != cur.right || new_rc.bottom != cur.bottom {
                        unsafe { ClipCursor(&new_rc) };
                        current_rect = Some(new_rc);
                        clipped = true;
                        release_on_exit = false;
                        println!("F11 pressed: Changed lock to new monitor");
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(16)); // ~60Hz
    }
}
