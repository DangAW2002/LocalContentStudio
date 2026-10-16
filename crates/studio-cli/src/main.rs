use clap::{Args, Parser, Subcommand};
use std::env;
use std::path::PathBuf;
use studio_workspace::{disk_usage, CheckpointService, Indexer, Org, Project, Workspace};

#[derive(Parser, Debug)]
#[command(name = "studio", about = "Local Content Studio CLI")]
struct Cli {
    /// Workspace directory (default: current working directory)
    #[arg(short, long)]
    workspace: Option<PathBuf>,

    /// Output results as JSON
    #[arg(long)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage organizations
    Org(OrgArgs),
    /// Manage workspaces
    Workspace(WorkspaceArgs),
    /// Manage projects
    Project(ProjectArgs),
    /// Manage checkpoints
    Checkpoint(CheckpointArgs),
    /// Manage indexing
    Index(IndexArgs),
    /// Display disk usage statistics
    Usage,
}

#[derive(Args, Debug)]
struct OrgArgs {
    #[command(subcommand)]
    cmd: OrgSubcommands,
}

#[derive(Subcommand, Debug)]
enum OrgSubcommands {
    /// Initialize a new organization layout
    Init {
        /// Target directory
        dir: PathBuf,
        /// Organization name
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Args, Debug)]
struct WorkspaceArgs {
    #[command(subcommand)]
    cmd: WorkspaceSubcommands,
}

#[derive(Subcommand, Debug)]
enum WorkspaceSubcommands {
    /// Initialize a new workspace layout
    Init {
        /// Target directory
        dir: PathBuf,
        /// Workspace display name
        #[arg(short, long)]
        name: String,
    },
    /// Open an existing workspace and report its details
    Open {
        /// Workspace directory
        dir: PathBuf,
    },
}

#[derive(Args, Debug)]
struct ProjectArgs {
    #[command(subcommand)]
    cmd: ProjectSubcommands,
}

#[derive(Subcommand, Debug)]
enum ProjectSubcommands {
    /// Create a new project inside the current workspace
    Create {
        /// Project slug
        slug: String,
        /// Optional project title (defaults to slug)
        #[arg(short, long)]
        title: Option<String>,
    },
}

#[derive(Args, Debug)]
struct CheckpointArgs {
    #[command(subcommand)]
    cmd: CheckpointSubcommands,
}

#[derive(Subcommand, Debug)]
enum CheckpointSubcommands {
    /// Create a new snapshot checkpoint
    Create {
        /// Label for the checkpoint
        label: String,
        /// Associated turn id
        #[arg(long)]
        turn_id: Option<String>,
    },
    /// List all checkpoints
    List,
    /// Restore workspace to a specific checkpoint
    Restore {
        /// Checkpoint id
        id: String,
    },
    /// Diff two checkpoints
    Diff {
        /// Checkpoint A ID
        a: String,
        /// Checkpoint B ID
        b: String,
    },
}

#[derive(Args, Debug)]
struct IndexArgs {
    #[command(subcommand)]
    cmd: IndexSubcommands,
}

#[derive(Subcommand, Debug)]
enum IndexSubcommands {
    /// Rebuild search and asset/artifact indexes from files
    Rebuild,
}

fn resolve_workspace(opt: Option<PathBuf>) -> anyhow::Result<Workspace> {
    let dir = match opt {
        Some(d) => d,
        None => env::current_dir()?,
    };
    Workspace::open(&dir)
        .map_err(|e| anyhow::anyhow!("Không thể mở workspace tại {}: {}", dir.display(), e))
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Org(org_args) => match org_args.cmd {
            OrgSubcommands::Init { dir, name } => {
                let org = Org::create(&dir, &name)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "root": org.root().display().to_string(),
                            "name": org.name(),
                            "is_implicit": org.is_implicit(),
                        })
                    );
                } else {
                    println!(
                        "Đã khởi tạo tổ chức '{}' tại: {}",
                        org.name(),
                        org.root().display()
                    );
                }
            }
        },
        Commands::Workspace(ws_args) => match ws_args.cmd {
            WorkspaceSubcommands::Init { dir, name } => {
                let ws = Workspace::create(&dir, &name)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "root": ws.root().display().to_string(),
                            "name": ws.name(),
                            "org": ws.org().name(),
                        })
                    );
                } else {
                    println!(
                        "Đã khởi tạo workspace '{}' tại: {}",
                        ws.name(),
                        ws.root().display()
                    );
                    println!("Tổ chức: {}", ws.org().name());
                }
            }
            WorkspaceSubcommands::Open { dir } => {
                let ws = Workspace::open(&dir)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "root": ws.root().display().to_string(),
                            "name": ws.name(),
                            "org": ws.org().name(),
                        })
                    );
                } else {
                    println!(
                        "Đã mở workspace '{}' tại: {}",
                        ws.name(),
                        ws.root().display()
                    );
                    println!("Tổ chức: {}", ws.org().name());
                }
            }
        },
        Commands::Project(proj_args) => match proj_args.cmd {
            ProjectSubcommands::Create { slug, title } => {
                let ws = resolve_workspace(cli.workspace)?;
                let title_str = title.unwrap_or_else(|| slug.clone());
                let proj = Project::create(&ws, &slug, &title_str)?;
                if cli.json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "slug": proj.slug(),
                            "title": proj.title(),
                            "root": proj.root().display().to_string(),
                        })
                    );
                } else {
                    println!(
                        "Đã tạo dự án '{}' (slug: {}) tại: {}",
                        proj.title(),
                        proj.slug(),
                        proj.root().display()
                    );
                }
            }
        },
        Commands::Checkpoint(cp_args) => match cp_args.cmd {
            CheckpointSubcommands::Create { label, turn_id } => {
                let ws = resolve_workspace(cli.workspace)?;
                let id = CheckpointService::create(&ws, &label, turn_id)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "id": id, "label": label }));
                } else {
                    println!("Đã tạo checkpoint '{}' thành công: {}", label, id);
                }
            }
            CheckpointSubcommands::List => {
                let ws = resolve_workspace(cli.workspace)?;
                let list = CheckpointService::list(&ws)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&list)?);
                } else {
                    println!("=== DANH SÁCH CHECKPOINT ({}) ===", list.len());
                    for cp in list {
                        let pin_marker = if cp.pinned { "[GHIM] " } else { "" };
                        println!(
                            "- {}{}: \"{}\" ({})",
                            pin_marker, cp.id, cp.label, cp.created_at
                        );
                    }
                }
            }
            CheckpointSubcommands::Restore { id } => {
                let ws = resolve_workspace(cli.workspace)?;
                CheckpointService::restore(&ws, &id)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "restored": id }));
                } else {
                    println!("Đã khôi phục thành công về checkpoint: {}", id);
                }
            }
            CheckpointSubcommands::Diff { a, b } => {
                let ws = resolve_workspace(cli.workspace)?;
                let diff = CheckpointService::diff(&ws, &a, &b)?;
                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&diff)?);
                } else {
                    println!("=== KHÁC BIỆT GIỮA {} VÀ {} ===", a, b);
                    println!("+ Đã thêm ({})", diff.added.len());
                    for p in &diff.added {
                        println!("    + {}", p);
                    }
                    println!("* Đã sửa ({})", diff.modified.len());
                    for p in &diff.modified {
                        println!("    * {}", p);
                    }
                    println!("- Đã xóa ({})", diff.deleted.len());
                    for p in &diff.deleted {
                        println!("    - {}", p);
                    }
                }
            }
        },
        Commands::Index(idx_args) => match idx_args.cmd {
            IndexSubcommands::Rebuild => {
                let ws = resolve_workspace(cli.workspace)?;
                Indexer::rebuild(&ws)?;
                if cli.json {
                    println!("{}", serde_json::json!({ "status": "rebuilt" }));
                } else {
                    println!("Đã tái tạo toàn bộ index và tìm kiếm FTS cho workspace thành công.");
                }
            }
        },
        Commands::Usage => {
            let ws = resolve_workspace(cli.workspace)?;
            let u = disk_usage(&ws)?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&u)?);
            } else {
                println!("=== THỐNG KÊ DUNG LƯỢNG WORKSPACE ===");
                println!("- Source files : {:>10} bytes", u.sources_bytes);
                println!("- Thư viện     : {:>10} bytes", u.library_bytes);
                println!("- Renders      : {:>10} bytes", u.renders_bytes);
                println!("- Cache        : {:>10} bytes", u.cache_bytes);
                println!("- Blobs (CAS)  : {:>10} bytes", u.blobs_bytes);
                println!("--------------------------------------");
                println!("Tổng cộng      : {:>10} bytes", u.total_bytes);
            }
        }
    }

    Ok(())
}
