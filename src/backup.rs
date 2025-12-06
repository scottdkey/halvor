use crate::config::EnvConfig;
use anyhow::{Context, Result};
use std::process::{Command, Stdio};
use std::time::SystemTime;

// New host-level backup functions
pub fn backup_host(hostname: &str, config: &EnvConfig) -> Result<()> {
    let host_config = config.hosts.get(hostname).with_context(|| {
        format!(
            "Host '{}' not found in .env\n\nAdd configuration to .env:\n  HOST_{}_IP=\"<ip-address>\"\n  HOST_{}_TAILSCALE=\"<tailscale-hostname>\"",
            hostname,
            hostname.to_uppercase(),
            hostname.to_uppercase()
        )
    })?;

    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(tailscale) = &host_config.tailscale {
        tailscale.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    let backup_base = format!("/mnt/smb/maple/backups/{}", hostname);

    println!(
        "Backing up all Docker volumes on {} ({})...",
        hostname, target_host
    );
    println!();

    let script = build_host_backup_script(hostname, &backup_base)?;
    execute_backup_script(&target_host, &script)?;

    println!();
    println!("✓ Backup complete for {}", hostname);

    Ok(())
}

pub fn list_backups(hostname: &str, config: &EnvConfig) -> Result<()> {
    let host_config = config
        .hosts
        .get(hostname)
        .with_context(|| format!("Host '{}' not found in .env", hostname))?;

    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(tailscale) = &host_config.tailscale {
        tailscale.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    let backup_base = format!("/mnt/smb/maple/backups/{}", hostname);

    println!("Listing backups for {} ({})...", hostname, target_host);
    println!();

    let script = build_list_host_backups_script(&backup_base)?;
    execute_backup_script(&target_host, &script)?;

    Ok(())
}

pub fn restore_host(hostname: &str, backup_name: Option<&str>, config: &EnvConfig) -> Result<()> {
    let host_config = config
        .hosts
        .get(hostname)
        .with_context(|| format!("Host '{}' not found in .env", hostname))?;

    let target_host = if let Some(ip) = &host_config.ip {
        ip.clone()
    } else if let Some(tailscale) = &host_config.tailscale {
        tailscale.clone()
    } else {
        anyhow::bail!("No IP or Tailscale hostname configured for {}", hostname);
    };

    let backup_base = format!("/mnt/smb/maple/backups/{}", hostname);

    if let Some(backup) = backup_name {
        println!("Restoring {} from backup '{}'...", hostname, backup);
        println!();

        let script = build_restore_host_script(hostname, &backup_base, backup)?;
        execute_backup_script(&target_host, &script)?;

        println!();
        println!("✓ Restore complete for {}", hostname);
    } else {
        // List backups and prompt
        println!("No backup name specified. Available backups:");
        println!();
        list_backups(hostname, config)?;
        println!();
        println!(
            "Use: hal backup {} restore --backup <backup-name>",
            hostname
        );
    }

    Ok(())
}

fn build_host_backup_script(hostname: &str, backup_base: &str) -> Result<String> {
    let datetime = chrono::DateTime::<chrono::Utc>::from(SystemTime::now());
    let timestamp_str = datetime.format("%Y%m%d_%H%M%S").to_string();
    let backup_dir = format!("{}/{}", backup_base, timestamp_str);

    let mut script = String::from("#!/bin/bash\nset -e\n\n");

    script.push_str(&format!(
        r#"
HOSTNAME="{}"
BACKUP_BASE="{}"
BACKUP_DIR="{}"
TIMESTAMP="{}"

echo "=== Host Backup Configuration ==="
echo "Host: $HOSTNAME"
echo "Backup Directory: $BACKUP_DIR"
echo ""

# Check if SMB mount exists (parent directory should be /mnt/smb/maple/backups)
SMB_MOUNT_BASE="/mnt/smb/maple/backups"
if [ ! -d "$SMB_MOUNT_BASE" ]; then
    echo "Error: SMB mount base directory $SMB_MOUNT_BASE does not exist or is not mounted"
    echo "Make sure SMB mount is set up: hal smb {} setup"
    exit 1
fi

# Create backup base directory if it doesn't exist
if [ ! -d "$BACKUP_BASE" ]; then
    echo "Creating backup base directory: $BACKUP_BASE"
    mkdir -p "$BACKUP_BASE"
    echo "✓ Created backup base directory"
fi

# Create backup directory
mkdir -p "$BACKUP_DIR"
echo "✓ Created backup directory: $BACKUP_DIR"

echo ""
echo "=== Stopping all containers ==="
# Stop all running containers
RUNNING_CONTAINERS=$(docker ps -q)
if [ -n "$RUNNING_CONTAINERS" ]; then
    RUNNING_CONTAINERS_COUNT=$(echo "$RUNNING_CONTAINERS" | wc -l)
    echo "Stopping $RUNNING_CONTAINERS_COUNT running container(s)..."
    docker stop $RUNNING_CONTAINERS || sudo docker stop $RUNNING_CONTAINERS
    echo "✓ All containers stopped"
else
    echo "✓ No running containers to stop"
fi

echo ""
echo "=== Backing up Docker volumes ==="
# Get all Docker volumes (try without sudo first, then with sudo)
VOLUMES=""
if command -v docker &> /dev/null; then
    VOLUMES=$(docker volume ls --format "{{{{.Name}}}}" 2>/dev/null || true)
fi
if [ -z "$VOLUMES" ] && command -v sudo &> /dev/null; then
    VOLUMES=$(sudo docker volume ls --format "{{{{.Name}}}}" 2>/dev/null || true)
fi

if [ -z "$VOLUMES" ]; then
    echo "No Docker volumes found"
else
    # Filter out empty lines and count
    VOL_COUNT=$(echo "$VOLUMES" | grep -v '^$' | wc -l)
    echo "Found $VOL_COUNT volume(s) to backup:"
    echo "$VOLUMES" | grep -v '^$' | sed 's/^/  - /'
    echo ""
    
    # Backup each volume
    for VOL in $(echo "$VOLUMES" | grep -v '^$'); do
        if [ -n "$VOL" ]; then
            echo "  Backing up volume: $VOL"
            # Try without sudo first
            if docker run --rm -v "$VOL":/data:ro -v "$BACKUP_DIR":/backup alpine tar czf "/backup/$VOL.tar.gz" -C /data . 2>&1; then
                echo "    ✓ Volume $VOL backed up"
            # Try with sudo if first attempt failed
            elif sudo docker run --rm -v "$VOL":/data:ro -v "$BACKUP_DIR":/backup alpine tar czf "/backup/$VOL.tar.gz" -C /data . 2>&1; then
                echo "    ✓ Volume $VOL backed up"
            else
                echo "    ✗ Failed to backup volume $VOL"
            fi
        fi
    done
fi

echo ""
echo "=== Backing up bind mounts from containers ==="
# Get all running and stopped containers with bind mounts
CONTAINERS=$(docker ps -a --format "{{{{.Names}}}}" 2>/dev/null || sudo docker ps -a --format "{{{{.Names}}}}" 2>/dev/null)

if [ -n "$CONTAINERS" ]; then
    for CONTAINER in $(echo "$CONTAINERS" | grep -v '^$'); do
        if [ -n "$CONTAINER" ]; then
            # Get bind mounts for this container
            BIND_MOUNTS=$(docker inspect "$CONTAINER" --format '{{{{range .Mounts}}}}{{{{if eq .Type "bind"}}}}{{{{.Source}}}}{{{{end}}}}{{{{end}}}}' 2>/dev/null || sudo docker inspect "$CONTAINER" --format '{{{{range .Mounts}}}}{{{{if eq .Type "bind"}}}}{{{{.Source}}}}{{{{end}}}}{{{{end}}}}' 2>/dev/null || true)
            
            if [ -n "$BIND_MOUNTS" ]; then
                for MOUNT_PATH in $(echo "$BIND_MOUNTS" | grep -v '^$'); do
                    if [ -n "$MOUNT_PATH" ] && [ -d "$MOUNT_PATH" ]; then
                        MOUNT_NAME=$(basename "$MOUNT_PATH" | tr '/' '_')
                        BACKUP_NAME="${{CONTAINER}}_${{MOUNT_NAME}}"
                        echo "  Backing up bind mount from ${{CONTAINER}}: ${{MOUNT_PATH}}"
                        if docker run --rm -v "${{MOUNT_PATH}}":/data:ro -v "${{BACKUP_DIR}}":/backup alpine tar czf "/backup/${{BACKUP_NAME}}.tar.gz" -C /data . 2>&1; then
                            echo "    ✓ Bind mount ${{MOUNT_PATH}} backed up as ${{BACKUP_NAME}}.tar.gz"
                        elif sudo docker run --rm -v "${{MOUNT_PATH}}":/data:ro -v "${{BACKUP_DIR}}":/backup alpine tar czf "/backup/${{BACKUP_NAME}}.tar.gz" -C /data . 2>&1; then
                            echo "    ✓ Bind mount ${{MOUNT_PATH}} backed up as ${{BACKUP_NAME}}.tar.gz"
                        else
                            echo "    ✗ Failed to backup bind mount $MOUNT_PATH"
                        fi
                    fi
                done
            fi
        fi
    done
else
    echo "No containers found"
fi

# Create a metadata file
cat > "$BACKUP_DIR/backup-info.txt" <<EOF
Host: $HOSTNAME
Timestamp: $TIMESTAMP
Date: $(date)
Volume Count: $(echo "$VOLUMES" | wc -l)
Volumes:
$(echo "$VOLUMES" | sed 's/^/  - /')
EOF
echo "✓ Created backup metadata"

echo ""
echo "=== Starting containers ==="
# Start containers back up
if [ -n "$RUNNING_CONTAINERS" ]; then
    echo "Starting containers..."
    docker start $RUNNING_CONTAINERS || sudo docker start $RUNNING_CONTAINERS
    echo "✓ Containers started"
else
    echo "✓ No containers to start"
fi

echo ""
echo "=== Backup Summary ==="
echo "Backup location: $BACKUP_DIR"
echo "Backup name: $TIMESTAMP"
echo "Volumes backed up: $(echo "$VOLUMES" | wc -l)"
ls -lh "$BACKUP_DIR" | tail -n +2
"#,
        hostname,
        backup_base,
        backup_dir,
        timestamp_str,
        hostname
    ));

    Ok(script)
}

fn build_list_host_backups_script(backup_base: &str) -> Result<String> {
    let mut script = String::from("#!/bin/bash\nset -e\n\n");

    script.push_str(&format!(
        r#"
BACKUP_BASE="{}"

echo "=== Available Backups ==="

if [ ! -d "$BACKUP_BASE" ]; then
    echo "Error: Backup directory $BACKUP_BASE does not exist or is not mounted"
    exit 1
fi

BACKUP_COUNT=$(find "$BACKUP_BASE" -mindepth 1 -maxdepth 1 -type d | wc -l)

if [ "$BACKUP_COUNT" -eq 0 ]; then
    echo "No backups found"
    exit 0
fi

echo "Found $BACKUP_COUNT backup(s):"
echo ""

for BACKUP in "$BACKUP_BASE"/*; do
    if [ -d "$BACKUP" ]; then
        BACKUP_NAME=$(basename "$BACKUP")
        BACKUP_DATE=$(stat -c %y "$BACKUP" 2>/dev/null || stat -f %Sm "$BACKUP" 2>/dev/null || echo "unknown")
        echo "  - $BACKUP_NAME"
        echo "    Date: $BACKUP_DATE"
        if [ -f "$BACKUP/backup-info.txt" ]; then
            echo "    Info:"
            cat "$BACKUP/backup-info.txt" | sed 's/^/      /'
        fi
        echo ""
    fi
done
"#,
        backup_base
    ));

    Ok(script)
}

fn build_restore_host_script(
    hostname: &str,
    backup_base: &str,
    backup_name: &str,
) -> Result<String> {
    let backup_dir = format!("{}/{}", backup_base, backup_name);

    let mut script = String::from("#!/bin/bash\nset -e\n\n");

    script.push_str(&format!(
        r#"
HOSTNAME="{}"
BACKUP_BASE="{}"
BACKUP_NAME="{}"
BACKUP_DIR="{}"

echo "=== Restore Configuration ==="
echo "Host: $HOSTNAME"
echo "Backup: $BACKUP_NAME"
echo "Backup Directory: $BACKUP_DIR"
echo ""

# Check if SMB mount exists (parent directory should be /mnt/smb/maple/backups)
SMB_MOUNT_BASE="/mnt/smb/maple/backups"
if [ ! -d "$SMB_MOUNT_BASE" ]; then
    echo "Error: SMB mount base directory $SMB_MOUNT_BASE does not exist or is not mounted"
    echo "Make sure SMB mount is set up: hal smb <hostname> setup"
    exit 1
fi

# Create backup base directory if it doesn't exist
if [ ! -d "$BACKUP_BASE" ]; then
    mkdir -p "$BACKUP_BASE"
    echo "Created backup base directory: $BACKUP_BASE"
fi

# Check if backup exists
if [ ! -d "$BACKUP_DIR" ]; then
    echo "Error: Backup directory does not exist: $BACKUP_DIR"
    echo "Available backups:"
    ls -1 "$BACKUP_BASE" 2>/dev/null || echo "  (none)"
    exit 1
fi

echo ""
echo "=== Stopping all containers ==="
# Stop all running containers
RUNNING_CONTAINERS=$(docker ps -q)
if [ -n "$RUNNING_CONTAINERS" ]; then
    RUNNING_CONTAINERS_COUNT=$(echo "$RUNNING_CONTAINERS" | wc -l)
    echo "Stopping $RUNNING_CONTAINERS_COUNT running container(s)..."
    docker stop $RUNNING_CONTAINERS || sudo docker stop $RUNNING_CONTAINERS
    echo "✓ All containers stopped"
else
    echo "✓ No running containers to stop"
fi

echo ""
echo "=== Restoring Docker volumes ==="

# Restore volumes from backup files
for BACKUP_FILE in "$BACKUP_DIR"/*.tar.gz; do
    if [ -f "$BACKUP_FILE" ]; then
        VOL_NAME=$(basename "$BACKUP_FILE" .tar.gz)
        echo "Restoring volume: $VOL_NAME"
        
        # Check if volume exists, create if not
        if ! docker volume inspect "$VOL_NAME" &>/dev/null && ! sudo docker volume inspect "$VOL_NAME" &>/dev/null; then
            docker volume create "$VOL_NAME" || sudo docker volume create "$VOL_NAME"
            echo "  Created volume: $VOL_NAME"
        fi
        
        # Restore volume
        if docker run --rm -v "$VOL_NAME":/data -v "$BACKUP_DIR":/backup alpine sh -c "cd /data && rm -rf * && tar xzf /backup/$VOL_NAME.tar.gz" 2>/dev/null; then
            echo "  ✓ Restored volume: $VOL_NAME"
        elif sudo docker run --rm -v "$VOL_NAME":/data -v "$BACKUP_DIR":/backup alpine sh -c "cd /data && rm -rf * && tar xzf /backup/$VOL_NAME.tar.gz" 2>/dev/null; then
            echo "  ✓ Restored volume: $VOL_NAME"
        else
            echo "  ✗ Failed to restore volume: $VOL_NAME"
        fi
    fi
done

echo ""
echo "=== Starting containers ==="
# Start containers back up
if [ -n "$RUNNING_CONTAINERS" ]; then
    echo "Starting containers..."
    docker start $RUNNING_CONTAINERS || sudo docker start $RUNNING_CONTAINERS
    echo "✓ Containers started"
else
    echo "✓ No containers to start"
fi

echo ""
echo "=== Restore Summary ==="
echo "Restored from: $BACKUP_DIR"
echo "Host: $HOSTNAME"
"#,
        hostname,
        backup_base,
        backup_name,
        backup_dir
    ));

    Ok(script)
}

fn execute_backup_script(host: &str, script: &str) -> Result<()> {
    use std::io::Write;

    let default_user = crate::config::get_default_username();
    let host_with_user = format!("{}@{}", default_user, host);

    // Test if key-based auth works
    let test_cmd = format!(
        r#"ssh -o ConnectTimeout=1 -o BatchMode=yes -o PreferredAuthentications=publickey -o PasswordAuthentication=no -o StrictHostKeyChecking=no {} 'echo test' >/dev/null 2>&1"#,
        host_with_user
    );

    let test_status = Command::new("sh")
        .arg("-c")
        .arg(&test_cmd)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    let use_key_auth = test_status.is_ok() && test_status.unwrap().success();

    let temp_script_path = format!("/tmp/hal-backup-{}.sh", std::process::id());

    // Write script to remote file
    let mut write_cmd = Command::new("ssh");
    if use_key_auth {
        write_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            &host_with_user,
            "bash",
            "-c",
            &format!(
                "cat > {} && chmod +x {}",
                temp_script_path, temp_script_path
            ),
        ]);
    } else {
        write_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            &host_with_user,
            "bash",
            "-c",
            &format!(
                "cat > {} && chmod +x {}",
                temp_script_path, temp_script_path
            ),
        ]);
    }

    write_cmd.stdin(Stdio::piped());
    write_cmd.stdout(Stdio::null());
    write_cmd.stderr(Stdio::inherit());

    let mut write_child = write_cmd.spawn()?;
    if let Some(mut stdin) = write_child.stdin.take() {
        stdin.write_all(script.as_bytes())?;
        stdin.flush()?;
        drop(stdin);
    }

    let write_status = write_child.wait()?;
    if !write_status.success() {
        anyhow::bail!("Failed to write backup script to remote system");
    }

    // Execute the script
    let mut exec_cmd = Command::new("ssh");
    if use_key_auth {
        exec_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey",
            "-o",
            "PasswordAuthentication=no",
            "-tt",
            &host_with_user,
            "bash",
            &temp_script_path,
        ]);
    } else {
        exec_cmd.args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "PreferredAuthentications=publickey,keyboard-interactive,password",
            "-tt",
            &host_with_user,
            "bash",
            &temp_script_path,
        ]);
    }

    exec_cmd.stdin(Stdio::inherit());
    exec_cmd.stdout(Stdio::inherit());
    exec_cmd.stderr(Stdio::inherit());

    let status = exec_cmd.status()?;

    // Clean up
    let _ = Command::new("ssh")
        .args([
            "-o",
            "StrictHostKeyChecking=no",
            "-o",
            "BatchMode=yes",
            &host_with_user,
            "rm",
            "-f",
            &temp_script_path,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if !status.success() {
        anyhow::bail!(
            "Backup script failed with exit code: {}",
            status.code().unwrap_or(1)
        );
    }

    Ok(())
}
