use clap::{Args, Parser, Subcommand};
use std::path::PathBuf;
use studio_app::{
    ApplicationHost, AttestRightsInput, CreateProjectInput, DeleteAssetInput, ImportAssetInput,
    IngestUrlInput, RevokeRightsInput, TransferScopeInput,
};
use studio_core::{
    actor::ActorContext,
    asset::{AssetKind, AssetScope},
    capabilities::CapabilityRegistry,
    operation::OperationEnvelope,
    project::ProjectType,
};
use studio_storage::{config::StorageConfig, initialize_storage};

#[derive(Parser, Debug)]
#[command(name = "studio", about = "Local Content Studio CLI")]
struct Cli {
    #[arg(long, default_value = "default-workspace")]
    workspace: String,

    #[arg(long)]
    proof: Option<String>,

    #[arg(long)]
    json: bool,

    #[arg(long)]
    data_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Project(ProjectArgs),
    Asset(AssetArgs),
    Research(ResearchArgs),
    Delivery(DeliveryArgs),
    Proof(ProofArgs),
    Capabilities,
}

#[derive(Args, Debug)]
struct ProofArgs {
    #[command(subcommand)]
    cmd: ProofSubcommands,
}

#[derive(Subcommand, Debug)]
enum ProofSubcommands {
    Issue {
        #[arg(short, long)]
        operation: String,
        #[arg(short, long)]
        target_id: String,
    },
}

#[derive(Args, Debug)]
struct DeliveryArgs {
    #[command(subcommand)]
    cmd: DeliverySubcommands,
}

#[derive(Subcommand, Debug)]
enum DeliverySubcommands {
    Ready {
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        revision_id: String,
        #[arg(long)]
        page_id: String,
    },
    Prepare {
        #[arg(long)]
        project_id: String,
        #[arg(long)]
        revision_id: String,
        #[arg(long, default_value = "facebook")]
        platform: String,
        #[arg(long)]
        caption: String,
        #[arg(long)]
        page_id: String,
    },
    Confirm {
        #[arg(long)]
        delivery_id: String,
    },
}

#[derive(Args, Debug)]
struct ResearchArgs {
    #[command(subcommand)]
    cmd: ResearchSubcommands,
}

#[derive(Subcommand, Debug)]
enum ResearchSubcommands {
    List,
    Ingest {
        #[arg(short, long)]
        url: String,
        #[arg(short, long)]
        file: Option<PathBuf>,
    },
    Get {
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Args, Debug)]
struct ProjectArgs {
    #[command(subcommand)]
    cmd: ProjectSubcommands,
}

#[derive(Subcommand, Debug)]
enum ProjectSubcommands {
    List,
    Create {
        #[arg(short, long)]
        title: String,
        #[arg(short = 'k', long, default_value = "poster")]
        project_type: String,
    },
    Get {
        #[arg(short, long)]
        id: String,
    },
}

#[derive(Args, Debug)]
struct AssetArgs {
    #[command(subcommand)]
    cmd: AssetSubcommands,
}

#[derive(Subcommand, Debug)]
enum AssetSubcommands {
    List,
    Import {
        #[arg(short, long)]
        file: PathBuf,
        #[arg(short, long)]
        name: Option<String>,
    },
    Attest {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        source: String,
        #[arg(short, long)]
        license: String,
    },
    Revoke {
        #[arg(short, long)]
        id: String,
        #[arg(short, long)]
        reason: String,
    },
    Transfer {
        #[arg(short, long)]
        id: String,
        #[arg(short = 'g', long)]
        global: bool,
    },
    Delete {
        #[arg(short, long)]
        id: String,
    },
}

fn resolve_data_dir(custom: Option<PathBuf>) -> PathBuf {
    if let Some(p) = custom {
        return p;
    }
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data).join("LocalContentStudio")
    } else {
        PathBuf::from(".").join(".studio-data")
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let data_dir = resolve_data_dir(cli.data_dir);
    let config = StorageConfig::new(data_dir);
    let (catalog, blob_storage, _) = initialize_storage(&config)?;
    let caps = CapabilityRegistry::default();
    let host = ApplicationHost::new(catalog, blob_storage, caps);

    // Authority: Identity & Workspace derived from Host authentication of OS user
    let session = host.authenticate_local_os_user(&cli.workspace)?;
    let actor = ActorContext {
        actor_id: session.subject_id.clone(),
        role: session.role,
        active_workspace_id: session.active_workspace_id.clone(),
        grant_token: None,
    };

    match cli.command {
        Commands::Project(proj_args) => match proj_args.cmd {
            ProjectSubcommands::List => {
                let list = host.list_projects(&actor)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&list)?);
                } else {
                    println!("=== DANH SÁCH DỰ ÁN ({}) ===", list.len());
                    for p in list {
                        println!(
                            "- [{}] {} ({:?}) - cập nhật: {}",
                            p.id, p.title, p.project_type, p.updated_at
                        );
                    }
                }
            }
            ProjectSubcommands::Create {
                title,
                project_type,
            } => {
                let p_type = match project_type.to_lowercase().as_str() {
                    "carousel" => ProjectType::Carousel,
                    "report" => ProjectType::Report,
                    "motion" => ProjectType::Motion,
                    _ => ProjectType::Poster,
                };
                let op = OperationEnvelope::new(
                    "project.create",
                    CreateProjectInput {
                        title: title.clone(),
                        project_type: p_type,
                    },
                );
                let res = host.create_project(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Tạo dự án thành công: {} (ID: {})",
                        res.result.title, res.result.id
                    );
                }
            }
            ProjectSubcommands::Get { id } => {
                let p = host.get_project(&actor, &id)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&p)?);
                } else {
                    println!("Dự án: {} (ID: {})", p.title, p.id);
                    println!("Trạng thái: {:?}, Loại: {:?}", p.status, p.project_type);
                }
            }
        },
        Commands::Asset(asset_args) => match asset_args.cmd {
            AssetSubcommands::List => {
                let list = host.list_assets(&actor)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&list)?);
                } else {
                    println!("=== THƯ VIỆN TÀI NGUYÊN ({}) ===", list.len());
                    for a in list {
                        println!(
                            "- [{}] {} ({}) - Bản quyền: {:?}",
                            a.id, a.original_name, a.verified_mime, a.rights_state
                        );
                    }
                }
            }
            AssetSubcommands::Import { file, name } => {
                let bytes = std::fs::read(&file)?;
                let file_name =
                    name.unwrap_or_else(|| file.file_name().unwrap().to_string_lossy().to_string());
                let op = OperationEnvelope::new(
                    "asset.import",
                    ImportAssetInput {
                        scope: AssetScope::Workspace,
                        kind: AssetKind::Image,
                        original_name: file_name,
                        declared_mime: "image/png".into(),
                        verified_mime: "image/png".into(),
                        provenance: "Nhập qua CLI".into(),
                        bytes,
                    },
                );
                let res = host.import_asset_bytes(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Nhập tài nguyên thành công (Trạng thái: {:?}): ID {}",
                        res.result.rights_state, res.result.id
                    );
                }
            }
            AssetSubcommands::Attest {
                id,
                source,
                license,
            } => {
                let mut op = OperationEnvelope::new(
                    "rights.attest",
                    AttestRightsInput {
                        asset_id: id,
                        source,
                        license_basis: license,
                        attribution_text: None,
                        allowed_uses: vec!["marketing".into()],
                    },
                );
                if let Some(ref p) = cli.proof {
                    op = op.with_human_proof(p);
                }
                let res = host.attest_asset_rights(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Xác nhận bản quyền thành công: Trạng thái mới: {:?}",
                        res.result.rights_state
                    );
                }
            }
            AssetSubcommands::Revoke { id, reason } => {
                let mut op = OperationEnvelope::new(
                    "rights.revoke",
                    RevokeRightsInput {
                        asset_id: id,
                        reason,
                    },
                );
                if let Some(ref p) = cli.proof {
                    op = op.with_human_proof(p);
                }
                let res = host.revoke_asset_rights(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Thu hồi quyền thành công: Trạng thái mới: {:?}",
                        res.result.rights_state
                    );
                }
            }
            AssetSubcommands::Transfer { id, global } => {
                let target_scope = if global {
                    AssetScope::Global
                } else {
                    AssetScope::Workspace
                };
                let mut op = OperationEnvelope::new(
                    "asset.transfer_scope",
                    TransferScopeInput {
                        asset_id: id,
                        target_scope,
                        target_workspace_id: None,
                    },
                );
                if let Some(ref p) = cli.proof {
                    op = op.with_human_proof(p);
                }
                let res = host.transfer_asset_scope(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Chuyển đổi phạm vi thành công: ID mới: {} (Trạng thái: {:?})",
                        res.result.id, res.result.rights_state
                    );
                }
            }
            AssetSubcommands::Delete { id } => {
                let mut op =
                    OperationEnvelope::new("asset.delete", DeleteAssetInput { asset_id: id });
                if let Some(ref p) = cli.proof {
                    op = op.with_human_proof(p);
                }
                let res = host.delete_asset(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!("Đã xóa tài nguyên: ID {}", res.result);
                }
            }
        },
        Commands::Research(research_args) => match research_args.cmd {
            ResearchSubcommands::List => {
                let list = host.list_research_sources(&actor)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&list)?);
                } else {
                    println!("=== DANH SÁCH TÀI LIỆU NGHIÊN CỨU ({}) ===", list.len());
                    for s in list {
                        let title = s
                            .snapshot
                            .as_ref()
                            .map(|x| x.title.as_str())
                            .unwrap_or("Chưa có tiêu đề");
                        println!(
                            "- [{}] {} ({}) - Trạng thái: {}",
                            s.id, title, s.canonical_url, s.fetch_status
                        );
                    }
                }
            }
            ResearchSubcommands::Ingest { url, file } => {
                let op = OperationEnvelope::new("research.ingest_url", IngestUrlInput { url });
                let res = if let Some(path) = file {
                    let html_content = std::fs::read_to_string(path)?;
                    host.ingest_url_content(&actor, op, &html_content)?
                } else {
                    // Invariant: validated pre-flight DNS, SSRF prevention, streamed 10MB byte cap
                    host.fetch_and_ingest_url(&actor, op, 10 * 1024 * 1024)?
                };

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Nhập tài liệu nghiên cứu thành công: [{}] {}",
                        res.result.id, res.result.canonical_url
                    );
                    if let Some(snap) = &res.result.snapshot {
                        println!("Tiêu đề: {}", snap.title);
                        println!(
                            "Đoạn chứng cứ trích xuất: {} đoạn",
                            snap.evidence_spans.len()
                        );
                    }
                }
            }
            ResearchSubcommands::Get { id } => {
                let s = host.get_research_source(&actor, &id)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&s)?);
                } else {
                    println!("Tài liệu: ID {}", s.id);
                    println!("URL: {}", s.canonical_url);
                    if let Some(snap) = &s.snapshot {
                        println!("Tiêu đề: {}", snap.title);
                        if let Some(desc) = &snap.description {
                            println!("Mô tả: {}", desc);
                        }
                        println!("Số đoạn chứng cứ: {}", snap.evidence_spans.len());
                    }
                }
            }
        },
        Commands::Delivery(del) => match del.cmd {
            DeliverySubcommands::Ready {
                project_id,
                revision_id,
                page_id,
            } => {
                let readiness =
                    host.check_publish_readiness(&actor, &project_id, &revision_id, &page_id, &[])?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&readiness)?);
                } else {
                    println!("=== KIỂM TRA ĐIỀU KIỆN XUẤT BẢN (PUBLISH READINESS) ===");
                    println!(
                        "Sẵn sàng: {}",
                        if readiness.is_ready { "CÓ" } else { "KHÔNG" }
                    );
                    println!("Target Page ID: {}", readiness.target_page_id);
                    println!("Source Hash: {}", readiness.source_hash);
                    if !readiness.blockers.is_empty() {
                        println!("Rào cản (Blockers):");
                        for b in &readiness.blockers {
                            println!("  * {}", b);
                        }
                    }
                    if !readiness.allowed_actions.is_empty() {
                        println!("Hành động khả dụng: {:?}", readiness.allowed_actions);
                    }
                }
            }
            DeliverySubcommands::Prepare {
                project_id,
                revision_id,
                platform,
                caption,
                page_id,
            } => {
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
                let res = host.prepare_delivery_package(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Gói DeliveryPackage đã tạo thành công: ID [{}] (Trạng thái: {:?})",
                        res.result.delivery_id, res.result.status
                    );
                    println!("Idempotency Key: {}", res.result.idempotency_key);
                }
            }
            DeliverySubcommands::Confirm { delivery_id } => {
                let mut op = OperationEnvelope::new(
                    "publish.confirm",
                    studio_app::ConfirmPublishInput { delivery_id },
                );
                if let Some(ref p) = cli.proof {
                    op = op.with_human_proof(p);
                }
                let res = host.confirm_publish_and_dispatch(&actor, op)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&res)?);
                } else {
                    println!(
                        "Xác nhận xuất bản và đưa vào Outbox thành công: ID [{}] (Đích: {})",
                        res.result.id, res.result.destination
                    );
                    println!("Trạng thái Outbox: {:?}", res.result.state);
                }
            }
        },
        Commands::Proof(proof_args) => match proof_args.cmd {
            ProofSubcommands::Issue {
                operation,
                target_id,
            } => {
                let action_hash = ApplicationHost::compute_action_hash(&target_id);
                let proof = host.issue_human_proof(&session, &operation, &action_hash)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&proof)?);
                } else {
                    println!("=== CẤP MÃ XÁC NHẬN NGƯỜI DÙNG (HUMAN PROOF) ===");
                    println!("Mã Proof: {}", proof.proof_id);
                    println!("Thao tác: {}", proof.operation);
                    println!("Thời hạn: đến {}", proof.expires_at);
                    println!(
                        "Sử dụng: truyền cờ `--proof {}` vào lệnh CLI tương ứng.",
                        proof.proof_id
                    );
                }
            }
        },
        Commands::Capabilities => {
            let caps = host.capabilities();
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&caps)?);
            } else {
                println!("=== BẢNG KHẢ NĂNG HỆ THỐNG (CAPABILITIES) ===");
                for (name, item) in &caps.capabilities {
                    println!(
                        "- {}: {:?} (Chỉ người dùng: {}) - {}",
                        name, item.status, item.human_only, item.description
                    );
                }
            }
        }
    }

    Ok(())
}
