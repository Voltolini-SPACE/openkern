//! The `kern` command-line entry point.
//!
//! A deliberately tiny surface: report the version, let orchestrators (`NOMOS`,
//! `OpenClaw`, `Hermes`, `CI`) query sandbox capabilities, and drive the governed context
//! engine (`kern context index|query|explain|stats`). Context output is structured and
//! `--json`-capable for integration.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::ExitCode;

use kern_context::types::Revision;
use kern_context::{ContextEngine, ContextQuery, Freshness, SourceKind, SymbolIndex};
use kern_exec::sandbox::{HostProcessGroupBackend, SandboxBackend};
use kern_repo::{Head, RepositoryIdentity};
use kern_types::MissionId;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("help", String::as_str);
    match cmd {
        "version" => {
            println!("openkern {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        "sandbox" => {
            print_sandbox_capabilities();
            ExitCode::SUCCESS
        }
        "context" => run_context(&args[1..]),
        _ => {
            print_usage();
            ExitCode::SUCCESS
        }
    }
}

fn print_usage() {
    println!("openkern {}", env!("CARGO_PKG_VERSION"));
    println!("usage:");
    println!("  kern version");
    println!("  kern sandbox");
    println!("  kern context index   <path>");
    println!("  kern context stats   <path>");
    println!("  kern context query   <path> <text...> [--json]");
    println!("  kern context explain <path> <text...>");
}

fn print_sandbox_capabilities() {
    let backend = HostProcessGroupBackend;
    let c = backend.capabilities();
    println!("backend={}", backend.name());
    println!("filesystem_isolation={}", c.filesystem_isolation);
    println!("network_deny_all={}", c.network_deny_all);
    println!("network_allowlist={}", c.network_allowlist);
    println!("pid_isolation={}", c.pid_isolation);
    println!("process_tree_kill={}", c.process_tree_kill);
    println!("environment_isolation={}", c.environment_isolation);
    println!("secret_isolation={}", c.secret_isolation);
}

fn run_context(args: &[String]) -> ExitCode {
    let sub = args.first().map_or("", String::as_str);
    let path = args.get(1).map_or(".", String::as_str);
    let json = args.iter().any(|a| a == "--json");
    let text: String = args
        .iter()
        .skip(2)
        .filter(|a| *a != "--json")
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");

    let index = match build_index(Path::new(path)) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    match sub {
        "index" | "stats" => {
            println!("repository={}", index.repository().as_str());
            println!("worktree={}", index.worktree().as_str());
            println!("revision={}", index.revision().as_str());
            println!("files={}", index.files().len());
            println!("symbols={}", index.symbols().len());
            println!("edges={}", index.graph().edges().len());
            ExitCode::SUCCESS
        }
        "query" | "explain" => run_query(&index, &text, json, sub == "explain"),
        _ => {
            print_usage();
            ExitCode::FAILURE
        }
    }
}

fn build_index(path: &Path) -> Result<SymbolIndex, String> {
    let identity = RepositoryIdentity::resolve(path).map_err(|e| e.to_string())?;
    let revision = match identity.head() {
        Head::Ref(r) => Revision(r.clone()),
        Head::Detached(o) => Revision(o.clone()),
    };
    SymbolIndex::build(
        identity.repository_id().clone(),
        identity.worktree_id().clone(),
        identity.worktree(),
        revision,
    )
    .map_err(|e| e.to_string())
}

fn run_query(index: &SymbolIndex, text: &str, json: bool, explain: bool) -> ExitCode {
    let mut allowed = BTreeSet::new();
    allowed.insert(SourceKind::Symbol);
    let query = ContextQuery {
        mission: MissionId::generate(),
        repository: index.repository().clone(),
        worktree: index.worktree().clone(),
        text: text.to_string(),
        seed_paths: Vec::new(),
        seed_symbols: Vec::new(),
        budget: kern_context::ContextBudget::default(),
        max_depth: 2,
        allowed_sources: allowed,
        freshness: Freshness::AnyRevision,
    };
    let mut engine = ContextEngine::new();
    let pack = match engine.build_pack(index, &query) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if json {
        print_pack_json(&pack);
    } else {
        println!("pack_id={}", pack.pack_id);
        println!("revision={}", pack.revision.as_str());
        println!(
            "items={} bytes={} tokens={}",
            pack.items.len(),
            pack.total_bytes,
            pack.total_estimated_tokens
        );
        for it in &pack.items {
            let p = it.provenance.path.to_string_lossy();
            println!("  {p}  score={:.2}  {}", it.score.total(), it.reason);
            if explain {
                let s = &it.score;
                println!(
                    "      lexical={:.2} symbol={:.2} dependency={:.2} path={:.2} explicit={:.2} test={:.2}",
                    s.lexical, s.symbol_match, s.dependency, s.path_proximity, s.explicit, s.test_relevance
                );
            }
        }
    }
    ExitCode::SUCCESS
}

fn print_pack_json(pack: &kern_context::ContextPack) {
    println!("{{");
    println!("  \"pack_id\": \"{}\",", esc(&pack.pack_id));
    println!("  \"pack_hash\": \"{}\",", esc(&pack.pack_hash));
    println!("  \"revision\": \"{}\",", esc(pack.revision.as_str()));
    println!("  \"engine_version\": \"{}\",", esc(&pack.engine_version));
    println!("  \"total_bytes\": {},", pack.total_bytes);
    println!(
        "  \"total_estimated_tokens\": {},",
        pack.total_estimated_tokens
    );
    println!("  \"items\": [");
    for (i, it) in pack.items.iter().enumerate() {
        let comma = if i + 1 < pack.items.len() { "," } else { "" };
        let symbol = it.provenance.symbol.as_ref().map_or("", |s| s.0.as_str());
        println!(
            "    {{ \"path\": \"{}\", \"symbol\": \"{}\", \"score\": {:.4}, \"tokens\": {}, \"reason\": \"{}\" }}{comma}",
            esc(&it.provenance.path.to_string_lossy()),
            esc(symbol),
            it.score.total(),
            it.estimated_tokens,
            esc(&it.reason),
        );
    }
    println!("  ]");
    println!("}}");
}

fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}
