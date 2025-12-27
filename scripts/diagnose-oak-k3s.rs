// Quick diagnostic script for oak K3s issues
// Run with: cargo run --bin diagnose-oak-k3s --manifest-path crates/halvor-agent/Cargo.toml

use halvor_agent::agent::{api::AgentClient, discovery::HostDiscovery};
use anyhow::{Context, Result};

fn main() -> Result<()> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Diagnosing K3s join issues on: oak");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();

    // Discover agents
    println!("1. Discovering halvor agents...");
    let discovery = HostDiscovery::default();
    let discovered_hosts = discovery.discover_all()
        .context("Failed to discover agents")?;
    
    println!("   Discovered {} agent(s)", discovered_hosts.len());
    for host in &discovered_hosts {
        println!("   - {} ({}:{})", host.hostname, 
            host.tailscale_ip.as_ref().or(host.local_ip.as_ref()).unwrap_or(&"unknown".to_string()),
            host.agent_port);
    }
    println!();

    // Find oak
    let oak = discovered_hosts.iter()
        .find(|h| {
            halvor_core::utils::hostname::normalize_hostname(&h.hostname) == "oak" ||
            h.hostname.eq_ignore_ascii_case("oak")
        })
        .ok_or_else(|| anyhow::anyhow!("Oak agent not found"))?;
    
    println!("2. Found oak agent at {}:{}", 
        oak.tailscale_ip.as_ref().or(oak.local_ip.as_ref()).unwrap_or(&"unknown".to_string()),
        oak.agent_port);
    
    let agent_ip = oak.tailscale_ip.as_ref()
        .or(oak.local_ip.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No IP for oak"))?;
    
    let client = AgentClient::new(agent_ip, oak.agent_port);
    
    // Test connection
    println!("3. Testing agent connection...");
    match client.ping() {
        Ok(true) => println!("   ✓ Agent is responding"),
        _ => {
            println!("   ✗ Agent is not responding");
            return Ok(());
        }
    }
    println!();

    // Run diagnostics
    let diagnostics = vec![
        ("K3s binary", "test -f /usr/local/bin/k3s && echo 'exists' || echo 'NOT FOUND'"),
        ("K3s service status", "systemctl is-active k3s 2>/dev/null || systemctl is-active k3s-agent 2>/dev/null || echo 'not running'"),
        ("Architecture", "uname -m"),
        ("OS", "uname -s"),
        ("Tailscale IP", "tailscale ip -4 2>/dev/null || echo 'not available'"),
        ("Ping frigg", "ping -c 1 -W 2 frigg.bombay-pinecone.ts.net 2>&1 | head -1 || echo 'failed'"),
    ];

    for (name, cmd) in diagnostics {
        println!("4. Checking {}...", name);
        match client.execute_command("sh", &["-c", cmd]) {
            Ok(output) => {
                let trimmed = output.trim();
                if trimmed.is_empty() {
                    println!("   (no output)");
                } else {
                    println!("   {}", trimmed);
                }
            }
            Err(e) => {
                println!("   ✗ Error: {}", e);
            }
        }
        println!();
    }

    // Get K3s logs
    println!("5. K3s service logs (last 30 lines)...");
    match client.execute_command("sh", &["-c", "journalctl -u k3s -n 30 --no-pager 2>/dev/null || journalctl -u k3s-agent -n 30 --no-pager 2>&1 || echo 'No logs found'"]) {
        Ok(output) => {
            if output.trim().is_empty() {
                println!("   (no logs)");
            } else {
                for line in output.lines().take(30) {
                    println!("   {}", line);
                }
            }
        }
        Err(e) => {
            println!("   ✗ Error: {}", e);
        }
    }
    println!();

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Diagnosis complete");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    Ok(())
}

