//! The `kern` command-line entry point.
//!
//! A deliberately tiny surface for the bootstrap: report the version and let external
//! orchestrators (`NOMOS`, `OpenClaw`, `Hermes`, `CI`) query what the sandbox can actually
//! enforce before they hand `OpenKern` a mission.

use kern_exec::sandbox::{HostProcessGroupBackend, SandboxBackend};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cmd = args.first().map_or("help", String::as_str);
    match cmd {
        "version" => println!("openkern {}", env!("CARGO_PKG_VERSION")),
        "sandbox" => print_sandbox_capabilities(),
        _ => {
            println!("openkern {}", env!("CARGO_PKG_VERSION"));
            println!("usage: kern <version|sandbox>");
            println!("  version   print the OpenKern version");
            println!("  sandbox   print the host sandbox backend capabilities");
        }
    }
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
