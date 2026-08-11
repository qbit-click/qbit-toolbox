// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if qbit_desktop_lib::run().is_err() {
        eprintln!("Qbit Toolbox failed to start.");
        std::process::exit(1);
    }
}
