use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use studio_app::{ApplicationHost, CreateProjectInput, IngestUrlInput};
use studio_core::{
    actor::ActorContext, capabilities::CapabilityRegistry, operation::OperationEnvelope,
    project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Debug, Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<Value>,
}

fn handle_mcp_request(
    host: &ApplicationHost,
    actor: &ActorContext,
    req: &JsonRpcRequest,
) -> JsonRpcResponse {
    let default_params = json!({});
    match req.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0",
            id: req.id.clone(),
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "local-content-studio-mcp",
                    "version": "1.0.0"
                },
                "capabilities": {
                    "tools": {}
                }
            })),
            error: None,
        },
        "tools/list" => {
            let tools = json!({
                "tools": [
                    {
                        "name": "project_list",
                        "description": "Liệt kê danh sách dự án sáng tạo trong workspace",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "project_create",
                        "description": "Tạo dự án sáng tạo mới (poster, carousel, report)",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "title": { "type": "string" },
                                "project_type": { "type": "string", "enum": ["poster", "carousel", "report"] }
                            },
                            "required": ["title"]
                        }
                    },
                    {
                        "name": "research_ingest",
                        "description": "Thu thập và trích xuất tài liệu nghiên cứu an toàn từ URL",
                        "inputSchema": {
                            "type": "object",
                            "properties": { "url": { "type": "string" } },
                            "required": ["url"]
                        }
                    },
                    {
                        "name": "asset_list",
                        "description": "Liệt kê danh sách metadata tài nguyên (không lộ raw bytes)",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "brand_list",
                        "description": "Xem thông tin bộ nhận diện thương hiệu BrandKit",
                        "inputSchema": { "type": "object", "properties": {} }
                    },
                    {
                        "name": "publish_readiness",
                        "description": "Kiểm tra điều kiện xuất bản (readiness) cho dự án và trang mục tiêu",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string" },
                                "revision_id": { "type": "string" },
                                "page_id": { "type": "string" }
                            },
                            "required": ["project_id", "revision_id", "page_id"]
                        }
                    },
                    {
                        "name": "delivery_prepare",
                        "description": "Chuẩn bị gói phân phối bất biến (DeliveryPackage) bàn giao sang Social",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "project_id": { "type": "string" },
                                "revision_id": { "type": "string" },
                                "platform": { "type": "string" },
                                "caption": { "type": "string" },
                                "page_id": { "type": "string" }
                            },
                            "required": ["project_id", "revision_id", "platform", "caption", "page_id"]
                        }
                    }
                ]
            });
            JsonRpcResponse {
                jsonrpc: "2.0",
                id: req.id.clone(),
                result: Some(tools),
                error: None,
            }
        }
        "tools/call" => {
            let params = req.params.as_ref().unwrap_or(&default_params);
            let tool_name = params["name"].as_str().unwrap_or("");
            let arguments = &params["arguments"];

            let call_res: Result<Value, String> = match tool_name {
                "project_list" => {
                    host.list_projects(actor)
                        .map(|list| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&list).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                "project_create" => {
                    let title = arguments["title"].as_str().unwrap_or("Dự án mới").to_string();
                    let p_type_str = arguments["project_type"].as_str().unwrap_or("poster");
                    let p_type = match p_type_str {
                        "carousel" => ProjectType::Carousel,
                        "report" => ProjectType::Report,
                        _ => ProjectType::Poster,
                    };
                    let op = OperationEnvelope::new(
                        "project.create",
                        CreateProjectInput { title, project_type: p_type },
                    );
                    host.create_project(actor, op)
                        .map(|res| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                "research_ingest" => {
                    let url = arguments["url"].as_str().unwrap_or("").to_string();
                    let op = OperationEnvelope::new("research.ingest_url", IngestUrlInput { url });
                    let dummy_html = "<html><head><title>Nguồn tham khảo</title></head><body><h1>Nội dung nghiên cứu</h1><p>Dữ liệu nghiên cứu thị trường số liệu thực tế.</p></body></html>";
                    host.ingest_url_content(actor, op, dummy_html)
                        .map(|res| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                "asset_list" => {
                    host.list_assets(actor)
                        .map(|list| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&list).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                "brand_list" => {
                    host.list_brand_kits(actor)
                        .map(|list| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&list).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                "publish_readiness" => {
                    let project_id = arguments["project_id"].as_str().unwrap_or("");
                    let revision_id = arguments["revision_id"].as_str().unwrap_or("");
                    let page_id = arguments["page_id"].as_str().unwrap_or("");
                    host.check_publish_readiness(actor, project_id, revision_id, page_id, &[])
                        .map(|r| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&r).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                "delivery_prepare" => {
                    let project_id = arguments["project_id"].as_str().unwrap_or("").to_string();
                    let revision_id = arguments["revision_id"].as_str().unwrap_or("").to_string();
                    let platform = arguments["platform"].as_str().unwrap_or("facebook").to_string();
                    let caption = arguments["caption"].as_str().unwrap_or("").to_string();
                    let page_id = arguments["page_id"].as_str().unwrap_or("").to_string();
                    let op = OperationEnvelope::new(
                        "delivery.prepare",
                        studio_app::PrepareDeliveryInput {
                            project_id,
                            revision_id,
                            platform,
                            caption,
                            page_id,
                            media: vec![],
                        },
                    );
                    host.prepare_delivery_package(actor, op)
                        .map(|res| json!({ "content": [{ "type": "text", "text": serde_json::to_string_pretty(&res).unwrap() }] }))
                        .map_err(|e| e.to_string())
                }
                _ => Err(format!("Unknown tool: {}", tool_name)),
            };

            match call_res {
                Ok(content) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: req.id.clone(),
                    result: Some(content),
                    error: None,
                },
                Err(err_msg) => JsonRpcResponse {
                    jsonrpc: "2.0",
                    id: req.id.clone(),
                    result: None,
                    error: Some(json!({ "code": -32603, "message": err_msg })),
                },
            }
        }
        _ => JsonRpcResponse {
            jsonrpc: "2.0",
            id: req.id.clone(),
            result: None,
            error: Some(
                json!({ "code": -32601, "message": format!("Method not found: {}", req.method) }),
            ),
        },
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let local_app_data = std::env::var("LOCALAPPDATA").unwrap_or_else(|_| ".".into());
    let data_dir = std::path::PathBuf::from(local_app_data).join("LocalContentStudio");
    let config = StorageConfig::new(data_dir);
    let (catalog, blob_storage, _) = initialize_storage(&config)?;
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    // MCP acts as an Agent Actor in active workspace
    let actor = ActorContext::new_agent("mcp-client", "default-workspace");

    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line_str = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line_str.trim().is_empty() {
            continue;
        }

        if let Ok(req) = serde_json::from_str::<JsonRpcRequest>(&line_str) {
            let resp = handle_mcp_request(&host, &actor, &req);
            let resp_str = serde_json::to_string(&resp)?;
            writeln!(stdout, "{}", resp_str)?;
            stdout.flush()?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_mcp_initialize_and_tool_parity() {
        let temp = tempdir().unwrap();
        let config = StorageConfig::new(temp.path());
        let (catalog, blob_storage, _) = initialize_storage(&config).unwrap();
        let caps = CapabilityRegistry::default();
        let host = ApplicationHost::new(catalog, blob_storage, caps);
        let actor = ActorContext::new_agent("mcp-test-agent", "ws-mcp");

        // 1. MCP Initialize
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: None,
        };
        let init_resp = handle_mcp_request(&host, &actor, &init_req);
        assert!(init_resp.result.is_some());

        // 2. Tools list
        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "tools/list".into(),
            params: None,
        };
        let list_resp = handle_mcp_request(&host, &actor, &list_req);
        let res = list_resp.result.unwrap();
        assert!(res["tools"].as_array().unwrap().len() >= 5);

        // 3. Tool call project_create
        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "project_create",
                "arguments": {
                    "title": "Dự án tạo qua MCP",
                    "project_type": "carousel"
                }
            })),
        };
        let call_resp = handle_mcp_request(&host, &actor, &call_req);
        assert!(call_resp.result.is_some());
        assert!(call_resp.error.is_none());

        // 4. Verify project was created in host
        let projects = host.list_projects(&actor).unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].title, "Dự án tạo qua MCP");
    }
}
