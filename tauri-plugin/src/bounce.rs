use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct BounceResult {
    pub success: bool,
    pub bounce_path: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BounceOptions {
    pub project_path: String,
    pub output_name: Option<String>,
    pub output_format: Option<String>,
}

#[tauri::command]
pub async fn auto_bounce_garageband(
    options: BounceOptions,
) -> Result<BounceResult, String> {
    #[cfg(not(target_os = "macos"))]
    {
        return Ok(BounceResult {
            success: false,
            bounce_path: None,
            error_message: Some("Auto bounce is only available on macOS".to_string()),
        });
    }

    #[cfg(target_os = "macos")]
    {
        let project_path = Path::new(&options.project_path);

        if !project_path.exists() {
            return Ok(BounceResult {
                success: false,
                bounce_path: None,
                error_message: Some("Project file does not exist".to_string()),
            });
        }

        let extension = project_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());
        let file_name = project_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.to_ascii_lowercase());
        let has_band_extension = extension.as_deref() == Some("band");
        let is_band_directory = project_path.is_dir()
            && file_name
                .as_deref()
                .map(|name| name.ends_with(".band"))
                .unwrap_or(false);
        let is_garageband_project = has_band_extension || is_band_directory;

        if !is_garageband_project {
            return Ok(BounceResult {
                success: false,
                bounce_path: None,
                error_message: Some("File is not a GarageBand project".to_string()),
            });
        }

        match execute_garageband_bounce(&options).await {
            Ok(bounce_path) => Ok(BounceResult {
                success: true,
                bounce_path: Some(bounce_path),
                error_message: None,
            }),
            Err(error) => Ok(BounceResult {
                success: false,
                bounce_path: None,
                error_message: Some(error),
            }),
        }
    }
}

#[cfg(target_os = "macos")]
async fn execute_garageband_bounce(options: &BounceOptions) -> Result<String, String> {
    use std::process::Command;
    use std::time::SystemTime;

    let format = options.output_format.as_deref().unwrap_or("aiff");
    let format_applescript = match format {
        "wav" => "Wave",
        "mp3" => "MP3",
        _ => "AIFF",
    };

    let project_path = Path::new(&options.project_path);
    let output_name = options
        .output_name
        .as_ref()
        .map(|n| n.clone())
        .unwrap_or_else(|| {
            project_path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        });

    let output_dir = project_path
        .parent()
        .ok_or("Could not determine output directory")?;

    let extension = match format {
        "wav" => "wav",
        "mp3" => "mp3",
        _ => "aiff",
    };

    let expected_bounce_path = output_dir
        .join(format!("{}.{}", output_name, extension))
        .to_string_lossy()
        .to_string();

    let script_path = get_bounce_script_path()?;
    let bounce_started_at = SystemTime::now();

    let output = Command::new("osascript")
        .arg(script_path)
        .arg(&options.project_path)
        .arg(&expected_bounce_path)
        .arg(format_applescript)
        .output()
        .map_err(|e| format!("Failed to execute osascript: {}", e))?;

    if !output.status.success() {
        let error_msg = String::from_utf8_lossy(&output.stderr);
        return Err(format!("AppleScript execution failed: {}", error_msg));
    }

    let result = String::from_utf8_lossy(&output.stdout);
    let result = result.trim();

    if let Some(err) = result.strip_prefix("ERR:") {
        return Err(err.trim().to_string());
    }

    if let Some(path_from_script) = result.strip_prefix("OK:") {
        let parsed_path = path_from_script.trim();
        if parsed_path == "DEFAULT_DIALOG_PATH" {
            let target_file_name = format!("{}.{}", output_name, extension);
            let resolved_path = resolve_default_dialog_output_path(
                &expected_bounce_path,
                &target_file_name,
                bounce_started_at,
            )?;
            wait_for_bounce_completion_since(&resolved_path, bounce_started_at).await?;
            return Ok(resolved_path);
        }
        if !parsed_path.is_empty() {
            wait_for_bounce_completion_since(parsed_path, bounce_started_at).await?;
            return Ok(parsed_path.to_string());
        }
    }

    wait_for_bounce_completion_since(&expected_bounce_path, bounce_started_at).await?;

    Ok(expected_bounce_path)
}

#[cfg(target_os = "macos")]
fn get_bounce_script_path() -> Result<PathBuf, String> {
    use std::io::Write;

    const SCRIPT_CONTENT: &str = include_str!("../scripts/garageband_bounce.applescript");

    let temp_path = std::env::temp_dir().join("slingshot_garageband_bounce.applescript");

    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| format!("Failed to create temp script file: {}", e))?;

    file.write_all(SCRIPT_CONTENT.as_bytes())
        .map_err(|e| format!("Failed to write temp script file: {}", e))?;

    Ok(temp_path)
}

#[cfg(target_os = "macos")]
async fn wait_for_bounce_completion_since(
    file_path: &str,
    modified_since: std::time::SystemTime,
) -> Result<(), String> {
    use std::time::{Duration, SystemTime};
    use tokio::time::sleep;

    let path = Path::new(file_path);

    let max_wait = Duration::from_secs(60);
    let poll_interval = Duration::from_millis(500);
    let start_time = SystemTime::now();

    loop {
        if start_time.elapsed().unwrap_or(Duration::ZERO) > max_wait {
            return Err("Timeout waiting for bounce to complete".to_string());
        }

        if path.exists() {
            if let Ok(metadata) = path.metadata() {
                if let Ok(modified) = metadata.modified() {
                    if modified >= modified_since {
                        sleep(Duration::from_millis(500)).await;
                        return Ok(());
                    }
                }
            }
        }

        sleep(poll_interval).await;
    }
}

#[cfg(target_os = "macos")]
fn resolve_default_dialog_output_path(
    expected_bounce_path: &str,
    target_file_name: &str,
    modified_since: std::time::SystemTime,
) -> Result<String, String> {
    use std::collections::HashSet;
    use std::fs;

    let expected_path = Path::new(expected_bounce_path);
    if let Ok(metadata) = fs::metadata(expected_path) {
        if let Ok(modified) = metadata.modified() {
            if modified >= modified_since {
                return Ok(expected_path.to_string_lossy().to_string());
            }
        }
    }

    let mut search_roots: Vec<PathBuf> = Vec::new();
    if let Some(parent) = expected_path.parent() {
        search_roots.push(parent.to_path_buf());
    }

    if let Ok(home_dir) = std::env::var("HOME") {
        let home = PathBuf::from(home_dir);
        search_roots.push(home.join("Desktop"));
        search_roots.push(home.join("Documents"));
        search_roots.push(home.join("Downloads"));
        search_roots.push(home.join("Music"));
        search_roots.push(home.join("Music").join("GarageBand"));
    }

    let mut seen = HashSet::new();
    let mut candidates: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for root in search_roots {
        if !root.exists() {
            continue;
        }

        let canonical = root.canonicalize().unwrap_or(root.clone());
        if !seen.insert(canonical) {
            continue;
        }

        collect_recent_matching_files(&root, target_file_name, modified_since, 0, 4, &mut candidates);
    }

    if candidates.is_empty() {
        return Err(format!(
            "Bounce completed but output file path could not be determined (expected '{}').",
            expected_bounce_path
        ));
    }

    candidates.sort_by(|a, b| b.0.cmp(&a.0));

    Ok(candidates[0].1.to_string_lossy().to_string())
}

#[cfg(target_os = "macos")]
fn collect_recent_matching_files(
    root: &Path,
    target_file_name: &str,
    modified_since: std::time::SystemTime,
    depth: usize,
    max_depth: usize,
    out: &mut Vec<(std::time::SystemTime, PathBuf)>,
) {
    use std::fs;

    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(file_type) => file_type,
            Err(_) => continue,
        };

        if file_type.is_dir() {
            if depth < max_depth {
                collect_recent_matching_files(
                    &path,
                    target_file_name,
                    modified_since,
                    depth + 1,
                    max_depth,
                    out,
                );
            }
            continue;
        }

        if !file_type.is_file() {
            continue;
        }

        let file_name_matches = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(target_file_name))
            .unwrap_or(false);
        if !file_name_matches {
            continue;
        }

        let modified = match fs::metadata(&path).and_then(|metadata| metadata.modified()) {
            Ok(modified) => modified,
            Err(_) => continue,
        };
        if modified >= modified_since {
            out.push((modified, path));
        }
    }
}

#[tauri::command]
pub async fn check_garageband_available() -> Result<bool, String> {
    #[cfg(not(target_os = "macos"))]
    {
        Ok(false)
    }

    #[cfg(target_os = "macos")]
    {
        use std::process::Command;

        if Path::new("/Applications/GarageBand.app").exists() {
            return Ok(true);
        }

        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"Finder\" to return exists application file \"GarageBand\"",
            ])
            .output()
            .map_err(|e| format!("Failed to check GarageBand: {}", e))?;

        if output.status.success() {
            let result = String::from_utf8_lossy(&output.stdout);
            Ok(result.trim() == "true")
        } else {
            Ok(false)
        }
    }
}
