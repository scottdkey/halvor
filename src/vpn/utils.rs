use anyhow::Result;

pub fn print_summary(
    hostname: &str,
    target_host: &str,
    all_passed: bool,
    error_output: &[u8],
) -> Result<()> {
    let error_output_str = String::from_utf8_lossy(error_output);
    if error_output_str.contains("No errors found") || error_output_str.trim().is_empty() {
        println!("   ✓ No recent errors in OpenVPN logs");
    } else {
        println!("   ⚠ Found potential issues in logs:");
        for line in error_output_str.lines().take(5) {
            if !line.trim().is_empty() && !line.contains("No errors found") {
                println!("     - {}", line.trim());
            }
        }
    }

    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    if all_passed {
        println!("  Host: {}", hostname);
        println!("  ✓ VPN Verification Complete - All Tests Passed");
    } else {
        println!("  ⚠ VPN Verification Complete - Some Tests Failed");
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!();
    println!(
        "VPN Status: {}",
        if all_passed {
            "OPERATIONAL"
        } else {
            "ISSUES DETECTED"
        }
    );
    println!();
    println!("Proxy Access:");
    println!("  From host: http://{}:8888", target_host);
    println!("  From containers: http://openvpn-pia:8888");
    println!();
    println!("Example usage:");
    println!(
        "  curl --proxy http://{}:8888 https://api.ipify.org",
        target_host
    );

    Ok(())
}
