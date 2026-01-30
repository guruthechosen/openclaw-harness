//! Status command - shows daemon status

pub async fn run() -> anyhow::Result<()> {
    println!("🛡️ MoltBot Harness Status");
    println!("─────────────────");
    
    // TODO: Check if daemon is running
    let running = false; // Placeholder
    
    if running {
        println!("Status: 🟢 Running");
        // TODO: Show more details
        // - Uptime
        // - Active collectors
        // - Recent actions count
        // - Critical alerts count
    } else {
        println!("Status: 🔴 Stopped");
        println!("\nRun 'openclaw-harness start' to start the daemon");
    }
    
    Ok(())
}
