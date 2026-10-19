use anyhow::Result;
use clap::{Parser, Subcommand};
use kiln_assets::{AssetCache, Archive};
use kiln_updater::{generate_patch, dlc::DlcMetadata};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn generate_steam_vdf() -> String {
    r#""appbuild"
{
    "appid" "YOUR_APP_ID"
    "desc" "My Game Build"
    "buildoutput" "./build_output"
    "contentroot" "./export"
    "setlive" "default"

    "depots"
    {
        "YOUR_DEPOT_ID"
        {
            "FileMapping"
            {
                "LocalPath" "*"
                "DepotPath" "."
                "recursive" "1"
            }
        }
    }
}"#.to_string()
}

#[derive(Parser)]
#[command(name = "kiln-cli")]
#[command(about = "Kiln asset pipeline CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Import files into the asset cache
    Import {
        /// Source directory to import
        #[arg(short, long)]
        source: PathBuf,
        /// Cache directory (default: .kiln-cache)
        #[arg(short, long, default_value = ".kiln-cache")]
        cache: PathBuf,
    },
    /// Pack cached files into an archive
    Pack {
        /// Source directory with files to pack
        #[arg(short, long)]
        source: PathBuf,
        /// Output archive path
        #[arg(short, long, default_value = "assets.kar")]
        output: PathBuf,
        /// Archive version (for V2 format)
        #[arg(long)]
        version: Option<String>,
    },
    /// Verify archive integrity
    Verify {
        /// Archive to verify
        #[arg(short, long)]
        archive: PathBuf,
    },
    /// Generate delta patch between two archives
    Patch {
        /// Old archive path
        #[arg(long)]
        old: PathBuf,
        /// New archive path
        #[arg(long)]
        new: PathBuf,
        /// Output patch file
        #[arg(short, long, default_value = "patch.qbsdiff")]
        output: PathBuf,
    },
    /// Export project for web (WASM build + HTML shell) or native platforms
    Export {
        /// Target platform: web, windows, macos, or linux
        #[arg(short, long, default_value = "web")]
        platform: String,
        /// Output directory for build
        #[arg(short, long, default_value = "export")]
        output: PathBuf,
        /// Create installers (for native platforms only)
        #[arg(long)]
        packaging: bool,
    },
    /// Publish to storefront (itch.io, Steam)
    Publish {
        /// Target storefront: itch or steam
        #[arg(short, long)]
        storefront: String,
        /// Project directory or build directory
        #[arg(short, long, default_value = "export")]
        build_dir: PathBuf,
    },
    /// Create DLC pack
    Dlc {
        /// Source directory with DLC files
        #[arg(short, long)]
        source: PathBuf,
        /// DLC ID
        #[arg(long)]
        id: String,
        /// DLC name
        #[arg(long)]
        name: String,
        /// DLC version
        #[arg(long)]
        version: String,
        /// Minimum game version required
        #[arg(long)]
        requires: String,
        /// Output archive path
        #[arg(short, long, default_value = "dlc.kar")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Import { source, cache } => {
            let mut asset_cache = AssetCache::new(cache);
            asset_cache.load()?;

            println!("Importing from: {}", source.display());
            let results = asset_cache.import_dir(&source)?;

            let imported = results.iter().filter(|r| matches!(r, kiln_assets::ImportResult::Imported { .. })).count();
            let skipped = results.iter().filter(|r| matches!(r, kiln_assets::ImportResult::Skipped { .. })).count();

            println!("Imported: {}, Skipped: {}", imported, skipped);
            asset_cache.save()?;
        }
        Commands::Pack { source, output, version } => {
            let files: Vec<PathBuf> = std::fs::read_dir(&source)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect();

            println!("Packing {} files into: {}", files.len(), output.display());
            let mut archive = Archive::new(output);

            if let Some(ver) = version {
                println!("Archive version: {}", ver);
                archive.pack_with_version(&files, Some(&ver))?;
            } else {
                archive.pack(&files)?;
            }

            println!("Archive created successfully");
        }
        Commands::Verify { archive } => {
            println!("Verifying: {}", archive.display());
            let loaded = Archive::load(&archive)?;
            let files = loaded.list();
            println!("Archive contains {} files:", files.len());
            for file in files {
                println!("  - {}", file);
            }

            // Check for V2 metadata
            if let Ok(Some(metadata)) = Archive::get_metadata(&archive) {
                println!("Archive version: {}", metadata.version);
                println!("Created: {}", metadata.created);
            }
        }
        Commands::Patch { old, new, output } => {
            println!("Generating patch from {} to {}...", old.display(), new.display());
            let (patch_data, metadata) = generate_patch(old.to_str().unwrap(), new.to_str().unwrap())?;

            fs::write(&output, patch_data)?;
            println!("Patch created: {}", output.display());
            println!("Patch size: {} bytes", metadata.patch_size);
            println!("Old hash: {}", metadata.old_hash);
            println!("New hash: {}", metadata.new_hash);
        }
        Commands::Dlc { source, id, name, version, requires, output } => {
            let files: Vec<PathBuf> = std::fs::read_dir(&source)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect();

            println!("Creating DLC pack: {} (v{})", name, version);
            println!("DLC ID: {}", id);
            println!("Requires game version: {}", requires);

            let mut archive = Archive::new(output.clone());
            archive.pack_with_version(&files, Some(&version))?;

            // Create DLC metadata file
            let dlc_metadata = DlcMetadata {
                id: id.clone(),
                name: name.clone(),
                version: version.clone(),
                requires_version: requires.clone(),
                dependencies: vec![],
            };

            let metadata_json = serde_json::to_string_pretty(&dlc_metadata)?;
            let metadata_path = output.with_extension("json");
            fs::write(&metadata_path, metadata_json)?;

            println!("DLC pack created: {}", output.display());
            println!("DLC metadata: {}", metadata_path.display());
        }
        Commands::Publish { storefront, build_dir } => {
            match storefront.as_str() {
                "itch" => {
                    println!("Publishing to itch.io from: {}", build_dir.display());

                    // Check if butler is available
                    let butler_check = Command::new("butler")
                        .arg("--version")
                        .output();

                    match butler_check {
                        Ok(_) => {
                            println!("butler CLI found");

                            // Documented manual flow if butler fails
                            println!("To publish manually:");
                            println!("1. Download butler from https://itch.io/docs/butler/");
                            println!("2. Login: butler login");
                            println!("3. Push build: butler push <build-dir> <user>/<game>:<channel>");
                        }
                        Err(_) => {
                            println!("butler CLI not found");
                            println!("Documented manual flow for itch.io:");
                            println!("1. Download butler from https://itch.io/docs/butler/");
                            println!("2. Install and add to PATH");
                            println!("3. Login: butler login");
                            println!("4. Push build: butler push <build-dir> <user>/<game>:<channel>");
                            println!("");
                            println!("Example:");
                            println!("  butler push export myusername/mygame:windows");
                        }
                    }
                }
                "steam" => {
                    println!("Publishing to Steam from: {}", build_dir.display());

                    // Generate depot/build scripts
                    println!("Generating Steam depot and build scripts...");

                    // Create build directory if it doesn't exist
                    fs::create_dir_all(&build_dir)?;

                    let vdf_content = generate_steam_vdf();
                    fs::write(build_dir.join("steam_build.vdf"), vdf_content)?;

                    println!("Steam build script generated: {}", build_dir.join("steam_build.vdf").display());
                    println!("To upload to Steam:");
                    println!("1. Install Steamworks SDK from Steamworks partner site");
                    println!("2. Edit steam_build.vdf and replace YOUR_APP_ID and YOUR_DEPOT_ID");
                    println!("3. Run: steamcmd +login <username> +run_app_build steam_build.vdf +quit");
                }
                _ => {
                    anyhow::bail!("Unknown storefront: {}. Supported: itch, steam", storefront);
                }
            }
        }
        Commands::Export { platform, output, packaging } => {
            match platform.as_str() {
                "web" => {
                    println!("Exporting to web build: {}", output.display());

                    // Create output directory
                    fs::create_dir_all(&output)?;

                    // Build WASM
                    println!("Building WASM...");
                    let build_status = Command::new("cargo")
                        .args([
                            "build",
                            "--target",
                            "wasm32-unknown-unknown",
                            "--release",
                            "-p",
                            "kiln-web",
                        ])
                        .status()?;

                    if !build_status.success() {
                        anyhow::bail!("WASM build failed");
                    }

                    // Run wasm-bindgen
                    println!("Running wasm-bindgen...");
                    let bindgen_status = Command::new("wasm-bindgen")
                        .args([
                            "target/wasm32-unknown-unknown/release/kiln_web.wasm",
                            "--out-dir",
                            &output.to_string_lossy(),
                            "--target",
                            "web",
                        ])
                        .status()?;

                    if !bindgen_status.success() {
                        anyhow::bail!("wasm-bindgen failed");
                    }

                    // Generate HTML shell
                    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kiln Web Export</title>
    <style>
        body {
            margin: 0;
            padding: 0;
            background: #0d0d0d;
            display: flex;
            justify-content: center;
            align-items: center;
            height: 100vh;
            overflow: hidden;
        }
        canvas {
            display: block;
        }
    </style>
</head>
<body>
    <canvas id="kiln-canvas"></canvas>
    <script type="module">
        import init from './kiln_web.js';
        init().then(() => {
            console.log('Kiln web runtime initialized');
        }).catch(err => {
            console.error('Failed to initialize:', err);
        });
    </script>
</body>
</html>"#;

                    fs::write(output.join("index.html"), html)?;

                    println!("Web export complete: {}", output.display());
                    println!("Open {}/index.html in a browser", output.display());
                }
                "windows" | "macos" | "linux" => {
                    println!("Exporting to {} build: {}", platform, output.display());

                    // Create output directory
                    fs::create_dir_all(&output)?;

                    // Determine target triple
                    let target = match platform.as_str() {
                        "windows" => "x86_64-pc-windows-msvc",
                        "macos" => "x86_64-apple-darwin",
                        "linux" => "x86_64-unknown-linux-gnu",
                        _ => anyhow::bail!("Unknown platform: {}", platform),
                    };

                    // Build player binary
                    println!("Building player binary for {}...", target);
                    let build_status = Command::new("cargo")
                        .args([
                            "build",
                            "--release",
                            "--target",
                            target,
                            "-p",
                            "kiln-player",
                        ])
                        .status()?;

                    if !build_status.success() {
                        anyhow::bail!("Player build failed for {}", target);
                    }

                    // Copy player binary to output
                    let binary_name = if platform == "windows" {
                        "kiln-player.exe"
                    } else {
                        "kiln-player"
                    };

                    let source_path = PathBuf::from(format!("target/{}/release/{}", target, binary_name));
                    let dest_path = output.join(binary_name);

                    fs::copy(&source_path, &dest_path)?;
                    println!("Player binary copied to: {}", dest_path.display());

                    // Create packaging if requested
                    if packaging {
                        match platform.as_str() {
                            "windows" => {
                                // Create zip archive for Windows
                                let zip_name = "game-windows.zip";
                                let zip_path = output.join(zip_name);
                                println!("Creating Windows zip archive...");

                                // For now, just zip the binary
                                // Full installer (MSI) requires WiX or cargo-dist with CI
                                let cmd_result = Command::new("powershell")
                                    .args([
                                        "-Command",
                                        &format!(
                                            "Compress-Archive -Path '{}' -DestinationPath '{}'",
                                            dest_path.display(),
                                            zip_path.display()
                                        ),
                                    ])
                                    .status();

                                match cmd_result {
                                    Ok(status) if status.success() => {
                                        println!("Windows archive created: {}", zip_path.display());
                                    }
                                    _ => {
                                        println!("Warning: Failed to create zip archive (requires PowerShell on Windows)");
                                    }
                                }
                            }
                            "macos" => {
                                // Create .app bundle for macOS
                                let app_name = "Game.app";
                                let app_path = output.join(app_name);
                                let contents_path = app_path.join("Contents");
                                let macos_path = contents_path.join("MacOS");
                                let resources_path = contents_path.join("Resources");

                                println!("Creating macOS .app bundle...");

                                fs::create_dir_all(&macos_path)?;
                                fs::create_dir_all(&resources_path)?;

                                // Copy binary to MacOS
                                let app_binary = macos_path.join("game");
                                fs::copy(&dest_path, &app_binary)?;
                                #[cfg(unix)]
                                {
                                    let mut perms = fs::metadata(&app_binary)?.permissions();
                                    perms.set_mode(0o755);
                                    fs::set_permissions(&app_binary, perms)?;
                                }

                                // Create Info.plist
                                let info_plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleExecutable</key>
    <string>game</string>
    <key>CFBundleIdentifier</key>
    <string>com.kiln.game</string>
    <key>CFBundleName</key>
    <string>Game</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleVersion</key>
    <string>0.1.0</string>
</dict>
</plist>"#;
                                fs::write(contents_path.join("Info.plist"), info_plist)?;

                                println!("macOS .app bundle created: {}", app_path.display());
                            }
                            "linux" => {
                                // Create AppImage for Linux
                                let appimage_name = "Game-x86_64.AppImage";
                                let appimage_path = output.join(appimage_name);
                                println!("Creating Linux AppImage...");

                                // Basic AppImage structure
                                let appdir = output.join("Game.AppDir");
                                let usr_bin = appdir.join("usr/bin");
                                fs::create_dir_all(&usr_bin)?;

                                // Copy binary
                                let app_binary = usr_bin.join("game");
                                fs::copy(&dest_path, &app_binary)?;
                                #[cfg(unix)]
                                {
                                    let mut perms = fs::metadata(&app_binary)?.permissions();
                                    perms.set_mode(0o755);
                                    fs::set_permissions(&app_binary, perms)?;
                                }

                                // Create .desktop file
                                let desktop_file = r#"[Desktop Entry]
Name=Game
Exec=game
Icon=game
Type=Application
Categories=Game;"#;
                                fs::write(appdir.join("Game.desktop"), desktop_file)?;

                                // Note: Full AppImage creation requires appimagetool
                                // For now, just create the AppDir structure
                                println!("Linux AppDir created: {}", appdir.display());
                                println!("Note: Use appimagetool to create final .AppImage:");
                                println!("  appimagetool {} {}", appdir.display(), appimage_path.display());
                            }
                            _ => {}
                        }
                    }

                    println!("Native export complete: {}", output.display());
                    println!("Player binary: {}", dest_path.display());
                    println!("Note: You must also pack game assets separately and provide them to the player");
                }
                _ => {
                    anyhow::bail!("Unknown platform: {}. Supported: web, windows, macos, linux", platform);
                }
            }
        }
    }

    Ok(())
}
