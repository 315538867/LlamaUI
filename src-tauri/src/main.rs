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

    // Default: launch GUI
    llamaui_lib::run()
}
