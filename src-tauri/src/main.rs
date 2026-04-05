// 在 release 构建隐藏 Windows 命令行窗口，MCP stdio 管道通信不受影响
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // CLI subcommand: `llamaui mcp` runs the MCP stdio server
    if args.len() > 1 && args[1] == "mcp" {
        let app_data_dir = dirs::data_dir()
            .map(|d| d.join("com.llamaui.app"))
            .expect("failed to get app data dir");
        std::fs::create_dir_all(&app_data_dir).ok();

        let config_store = Arc::new(llamaui_lib::services::config_store::ConfigStore::new(&app_data_dir));
        llamaui_lib::mcp::server::run_stdio_server(config_store);
        return;
    }

    // CLI subcommand: `llamaui proxy [--port PORT] [--target URL]`
    if args.len() > 1 && args[1] == "proxy" {
        let mut port: u16 = 8080;
        let mut target = "http://127.0.0.1:8000".to_string();
        let mut i = 2;
        while i < args.len() {
            match args[i].as_str() {
                "--port" if i + 1 < args.len() => {
                    port = args[i + 1].parse().unwrap_or(8080);
                    i += 2;
                }
                "--target" if i + 1 < args.len() => {
                    target = args[i + 1].clone();
                    i += 2;
                }
                _ => { i += 1; }
            }
        }
        llamaui_lib::proxy::server::run_proxy_server(port, &target);
        return;
    }

    // Default: launch GUI
    llamaui_lib::run()
}
