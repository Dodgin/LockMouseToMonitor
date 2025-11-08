# lockmousetomonitor

Small Windows-only Rust utility to keep the mouse locked to your chosen monitor. When you start the program, it will display a list of available monitors and let you choose which one to lock to. You can either select a specific monitor by number, or press Enter to use whichever monitor the cursor is currently on.

Controls:
- Press Ctrl to temporarily release the lock when your cursor reaches the monitor edge
- Once released, moving back to the locked monitor will re-engage the lock
- Press F11 while on a different monitor to switch which monitor is locked (useful for permanently changing monitors)

Build

Open a PowerShell prompt and run:

```powershell
cd 'c:\lockmousetomonitor'
cargo build --release
```

Run

Run the built executable from the release folder. No admin rights required.

Notes

- This uses the Win32 ClipCursor API to confine the cursor to a monitor rectangle.
- Behavior: normal operation locks to the monitor the cursor is on. Press Ctrl (either one) to set a "release on exit" state; when the cursor next reaches the monitor edge the program will release the clip and let you move to other monitors. When the cursor later returns to a monitor, the program re-applies the clip.
