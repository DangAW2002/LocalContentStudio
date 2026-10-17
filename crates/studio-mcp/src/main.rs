//! `studio-mcp --shim`: stdio MCP shim registered globally for agy (ADR-008 §4).

#[tokio::main(flavor = "current_thread")]
async fn main() -> std::io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--shim") {
        let path = args.get(1).map(std::path::PathBuf::from);
        return studio_mcp::shim::run(path).await;
    }
    eprintln!("Dùng: studio-mcp --shim [đường-dẫn-runtime.json]\nMCP server chạy bên trong ứng dụng Studio.");
    std::process::exit(2);
}
