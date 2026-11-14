// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(|s| s.as_str()) == Some("inspect") {
        std::process::exit(waypoint_lib::inspect_main(args[1..].to_vec()));
    }
    // 优化 WebView2 本地回环地址直连，防止代理软件劫持本地资产
    std::env::set_var(
        "WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS",
        "--proxy-bypass-list=<local>;*.localhost;tauri.localhost;127.0.0.1;127.*",
    );
    waypoint_lib::run()
}
