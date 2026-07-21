// Prevent a console window from appearing alongside the app on Windows release
// builds. Debug builds keep the console for logs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    carimbo_lib::run();
}
