use anyhow::Result;
use clap::{Parser, Subcommand};
use kiln_assets::{Archive, AssetCache};
use kiln_updater::{dlc::DlcMetadata, generate_patch};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Collect every file below a directory, recursively.
///
/// The project of a game holds its art and its scripts in folders, so the
/// whole tree is packed.
fn collect_files(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry.map_err(|e| anyhow::anyhow!("{e}"))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    files.sort();
    Ok(files)
}

/// Find a project template by name.
///
/// The templates live in the repository, so the CLI looks for them next to
/// the working directory, in the directory of the executable and in the
/// repository itself. That way `kiln-cli init` works from anywhere.
fn find_template(name: &str) -> Result<PathBuf> {
    let file_name = format!("{name}.ron");
    let mut candidates = vec![PathBuf::from("templates").join(&file_name)];

    if let Ok(exe) = std::env::current_exe() {
        // The templates live in the repository, so the search walks up from
        // the executable: `target/debug/kiln-cli` sits three levels below
        // the repository root.
        for dir in exe.ancestors().take(4) {
            candidates.push(dir.join("templates").join(&file_name));
        }
    }

    for candidate in &candidates {
        if candidate.is_file() {
            return Ok(candidate.clone());
        }
    }

    anyhow::bail!(
        "The template '{}' was not found. Looked in: {}",
        name,
        candidates
            .iter()
            .map(|path| path.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    )
}

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
}"#
    .to_string()
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
    /// Initialize a new project from a template
    Init {
        /// Project name
        #[arg(short, long)]
        name: String,
        /// Template: empty_2d, platformer, topdown
        #[arg(short, long, default_value = "empty_2d")]
        template: String,
        /// Output directory (default: project name)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
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
        /// Target platform: web, windows, macos, linux, android, or ios
        #[arg(short, long, default_value = "web")]
        platform: String,
        /// Output directory for build
        #[arg(short, long, default_value = "export")]
        output: PathBuf,
        /// Create installers (for native platforms only)
        #[arg(long)]
        packaging: bool,
        /// Create signed build for store (Android AAB, iOS requires Xcode)
        #[arg(long)]
        signed: bool,
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
    /// Validate build for store upload (Google Play, App Store)
    Validate {
        /// Target store: google_play or app_store
        #[arg(short, long)]
        store: String,
        /// Build directory to validate
        #[arg(short, long, default_value = "export")]
        build_dir: PathBuf,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init {
            name,
            template,
            output,
        } => {
            let output_dir = output.unwrap_or_else(|| PathBuf::from(&name));

            println!(
                "Initializing project '{}' from template '{}'...",
                name, template
            );
            println!("Output directory: {}", output_dir.display());

            // Map template name to file
            let template_name = match template.as_str() {
                "empty_2d" => "empty_2d",
                "platformer" => "platformer",
                "topdown" => "topdown",
                _ => anyhow::bail!(
                    "Unknown template: {}. Available: empty_2d, platformer, topdown",
                    template
                ),
            };
            let template_file = find_template(template_name)?;

            // Read template
            let template_content = fs::read_to_string(&template_file)?;

            // Parse as RON
            let mut project: kiln_core::Project = ron::from_str(&template_content)?;

            // Update project name
            project.name = name.clone();
            project.id = uuid::Uuid::new_v4().to_string();

            // Create output directory
            fs::create_dir_all(&output_dir)?;

            // Write project file
            let project_file = output_dir.join("kiln.toml");
            let toml_content = project
                .to_toml()
                .map_err(|e| anyhow::anyhow!("Failed to serialize project: {}", e))?;
            fs::write(&project_file, toml_content)?;

            println!("Project initialized: {}", project_file.display());
            println!("Run 'kiln-editor' to open the project");
        }
        Commands::Import { source, cache } => {
            let mut asset_cache = AssetCache::new(cache);
            asset_cache.load()?;

            println!("Importing from: {}", source.display());
            let results = asset_cache.import_dir(&source)?;

            let imported = results
                .iter()
                .filter(|r| matches!(r, kiln_assets::ImportResult::Imported { .. }))
                .count();
            let skipped = results
                .iter()
                .filter(|r| matches!(r, kiln_assets::ImportResult::Skipped { .. }))
                .count();

            println!("Imported: {}, Skipped: {}", imported, skipped);
            asset_cache.save()?;
        }
        Commands::Pack {
            source,
            output,
            version,
        } => {
            // The whole project is packed, so the art and the scripts end
            // up in the archive and the player can run it.
            let files = collect_files(&source)?;

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
            println!(
                "Generating patch from {} to {}...",
                old.display(),
                new.display()
            );
            let (patch_data, metadata) =
                generate_patch(old.to_str().unwrap(), new.to_str().unwrap())?;

            fs::write(&output, patch_data)?;
            println!("Patch created: {}", output.display());
            println!("Patch size: {} bytes", metadata.patch_size);
            println!("Old hash: {}", metadata.old_hash);
            println!("New hash: {}", metadata.new_hash);
        }
        Commands::Dlc {
            source,
            id,
            name,
            version,
            requires,
            output,
        } => {
            let files = collect_files(&source)?;

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
        Commands::Validate { store, build_dir } => {
            match store.as_str() {
                "google_play" => {
                    println!("Validating build for Google Play: {}", build_dir.display());

                    // Check for APK
                    let apk_path = build_dir.join("game.apk");
                    if !apk_path.exists() {
                        println!("Warning: APK not found at {}", apk_path.display());
                        println!("Expected: game.apk in build directory");
                    } else {
                        println!("✓ APK found: {}", apk_path.display());

                        // Check APK size (Play Store limit is 150 MB for initial upload)
                        let apk_size = fs::metadata(&apk_path)?.len();
                        println!("  APK size: {} MB", apk_size / 1024 / 1024);

                        if apk_size > 150 * 1024 * 1024 {
                            println!("  Warning: APK exceeds 150 MB Google Play limit");
                        }
                    }

                    // Check for AAB (required for Play Store)
                    let aab_path = build_dir.join("game.aab");
                    if !aab_path.exists() {
                        println!("Warning: AAB not found at {}", aab_path.display());
                        println!("Note: AAB is required for Play Store upload");
                        println!("See docs/ANDROID.md for AAB creation instructions");
                    } else {
                        println!("✓ AAB found: {}", aab_path.display());

                        let aab_size = fs::metadata(&aab_path)?.len();
                        println!("  AAB size: {} MB", aab_size / 1024 / 1024);
                    }

                    println!("\nGoogle Play validation checklist:");
                    println!("1. App signing key configured");
                    println!("2. Target SDK level is at least 33 (Android 13)");
                    println!("3. Permissions declared in AndroidManifest.xml");
                    println!("4. Privacy policy URL provided");
                    println!("5. Content rating questionnaire completed");
                    println!("6. Store listing prepared (screenshots, description, icon)");
                }
                "app_store" => {
                    println!("Validating build for App Store: {}", build_dir.display());

                    // iOS builds require Xcode on macOS
                    println!("Note: App Store validation requires macOS with Xcode");
                    println!(
                        "This CLI command performs basic checks; full validation requires Xcode"
                    );

                    // Check for iOS project structure
                    let ios_project = build_dir.join("ios");
                    if !ios_project.exists() {
                        println!(
                            "Warning: iOS project not found at {}",
                            ios_project.display()
                        );
                        println!(
                            "Run: kiln-cli export --platform ios to create iOS project structure"
                        );
                    } else {
                        println!("✓ iOS project structure found: {}", ios_project.display());

                        // Check for build script
                        let build_script = ios_project.join("build.sh");
                        if build_script.exists() {
                            println!("✓ Build script found: {}", build_script.display());
                        }
                    }

                    println!("\nApp Store validation checklist:");
                    println!("1. Apple Developer account enrolled");
                    println!("2. Bundle identifier unique (e.g., com.yourcompany.game)");
                    println!("3. Provisioning profile configured in Xcode");
                    println!("4. Code signing certificate installed");
                    println!("5. App icon and launch screen configured");
                    println!("6. Privacy descriptions for sensitive permissions");
                    println!("7. App Store Connect metadata prepared");
                    println!("8. TestFlight beta testing completed");
                    println!("9. App Review guidelines compliance verified");

                    println!("\nRequired Info.plist keys:");
                    println!("  - CFBundleIdentifier: Unique bundle ID");
                    println!("  - CFBundleVersion: Version number");
                    println!("  - CFBundleShortVersionString: Display version");
                    println!("  - UIRequiredDeviceCapabilities: Required capabilities");
                    println!("  - NSCameraUsageDescription: Camera permission (if used)");
                    println!("  - NSMicrophoneUsageDescription: Microphone permission (if used)");
                    println!(
                        "  - NSPhotoLibraryUsageDescription: Photo library permission (if used)"
                    );
                }
                _ => {
                    anyhow::bail!(
                        "Unknown store: {}. Supported: google_play, app_store",
                        store
                    );
                }
            }
        }
        Commands::Publish {
            storefront,
            build_dir,
        } => {
            match storefront.as_str() {
                "itch" => {
                    println!("Publishing to itch.io from: {}", build_dir.display());

                    // Check if butler is available
                    let butler_check = Command::new("butler").arg("--version").output();

                    match butler_check {
                        Ok(_) => {
                            println!("butler CLI found");

                            // Documented manual flow if butler fails
                            println!("To publish manually:");
                            println!("1. Download butler from https://itch.io/docs/butler/");
                            println!("2. Login: butler login");
                            println!(
                                "3. Push build: butler push <build-dir> <user>/<game>:<channel>"
                            );
                        }
                        Err(_) => {
                            println!("butler CLI not found");
                            println!("Documented manual flow for itch.io:");
                            println!("1. Download butler from https://itch.io/docs/butler/");
                            println!("2. Install and add to PATH");
                            println!("3. Login: butler login");
                            println!(
                                "4. Push build: butler push <build-dir> <user>/<game>:<channel>"
                            );
                            println!();
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

                    println!(
                        "Steam build script generated: {}",
                        build_dir.join("steam_build.vdf").display()
                    );
                    println!("To upload to Steam:");
                    println!("1. Install Steamworks SDK from Steamworks partner site");
                    println!("2. Edit steam_build.vdf and replace YOUR_APP_ID and YOUR_DEPOT_ID");
                    println!(
                        "3. Run: steamcmd +login <username> +run_app_build steam_build.vdf +quit"
                    );
                }
                _ => {
                    anyhow::bail!("Unknown storefront: {}. Supported: itch, steam", storefront);
                }
            }
        }
        Commands::Export {
            platform,
            output,
            packaging,
            signed,
        } => {
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
                    // Multithreading (SharedArrayBuffer / Workers with atomics) requires
                    // cross-origin isolation via COOP+COEP response headers. Meta tags
                    // cannot set these; serve with a local server that adds them.
                    let html = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Kiln Web Export</title>
    <!--
      For Web Workers / threads: serve this directory with headers:
        Cross-Origin-Opener-Policy: same-origin
        Cross-Origin-Embedder-Policy: require-corp
      Example: npx serve -l 8080 --cors  (plus a proxy that injects COOP/COEP)
      Or use the _headers file written next to this HTML for Netlify-style hosts.
      Without isolation, kiln-web automatically falls back to single-thread mode.
    -->
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
        import init, { run, is_cross_origin_isolated, can_use_web_workers } from './kiln_web.js';
        init().then(() => {
            console.log('Kiln web runtime initialized');
            console.log('crossOriginIsolated=', is_cross_origin_isolated());
            console.log('canUseWebWorkers=', can_use_web_workers());
            run();
        }).catch(err => {
            console.error('Failed to initialize:', err);
        });
    </script>
</body>
</html>"#;

                    fs::write(output.join("index.html"), html)?;

                    // Netlify / Cloudflare Pages style headers for COOP/COEP
                    let headers = "/*\n  Cross-Origin-Opener-Policy: same-origin\n  Cross-Origin-Embedder-Policy: require-corp\n";
                    fs::write(output.join("_headers"), headers)?;

                    println!("Web export complete: {}", output.display());
                    println!("Open {}/index.html in a browser", output.display());
                    println!("Note: for threads, serve with COOP/COEP (see _headers); else single-thread fallback.");
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

                    let source_path =
                        PathBuf::from(format!("target/{}/release/{}", target, binary_name));
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
                                println!(
                                    "  appimagetool {} {}",
                                    appdir.display(),
                                    appimage_path.display()
                                );
                            }
                            _ => {}
                        }
                    }

                    println!("Native export complete: {}", output.display());
                    println!("Player binary: {}", dest_path.display());
                    println!("Note: You must also pack game assets separately and provide them to the player");
                }
                "android" => {
                    println!("Exporting to Android build: {}", output.display());

                    // Create output directory
                    fs::create_dir_all(&output)?;

                    // Build APK using cargo-apk
                    println!("Building Android APK...");
                    let build_status = Command::new("cargo")
                        .args(["apk", "build", "--release", "-p", "kiln-player"])
                        .status()?;

                    if !build_status.success() {
                        anyhow::bail!("Android APK build failed");
                    }

                    // Copy APK to output
                    let apk_source = PathBuf::from(
                        "target/android/release/app/build/outputs/apk/release/app-release.apk",
                    );
                    let apk_dest = output.join("game.apk");

                    if apk_source.exists() {
                        fs::copy(&apk_source, &apk_dest)?;
                        println!("Android APK created: {}", apk_dest.display());
                    } else {
                        anyhow::bail!(
                            "APK not found at expected location: {}",
                            apk_source.display()
                        );
                    }

                    // Create signed AAB if requested
                    if signed {
                        println!("Note: Signed AAB requires manual keystore configuration");
                        println!("See docs/ANDROID.md for signing instructions");
                        println!(
                            "AAB creation is a manual process with Android Studio or bundletool"
                        );
                    }

                    println!("Android export complete: {}", output.display());
                }
                "ios" => {
                    println!("Exporting to iOS build: {}", output.display());

                    // iOS requires macOS and Xcode
                    println!("Note: iOS export requires macOS with Xcode installed");
                    println!("This CLI command creates the static library and project structure");
                    println!("You must open the Xcode project on macOS to complete the build and signing");

                    // Create output directory
                    fs::create_dir_all(&output)?;

                    // Determine target (device or simulator)
                    let target = "aarch64-apple-ios"; // Default to device

                    // Build static library for iOS
                    println!("Building iOS static library for {}...", target);
                    let build_status = Command::new("cargo")
                        .args([
                            "build",
                            "--release",
                            "--target",
                            target,
                            "-p",
                            "kiln-player",
                        ])
                        .status();

                    match build_status {
                        Ok(status) if status.success() => {
                            println!("iOS static library built successfully");
                        }
                        Ok(_) => {
                            anyhow::bail!(
                                "iOS build failed - this is expected on non-macOS platforms"
                            );
                        }
                        Err(e) => {
                            println!("Note: iOS build requires macOS and Xcode");
                            println!("Error: {}", e);
                            println!("To build on macOS:");
                            println!("1. Install Rust iOS targets: rustup target add aarch64-apple-ios aarch64-apple-ios-sim");
                            println!("2. Build: cargo build --release --target aarch64-apple-ios -p kiln-player");
                            println!("3. Create Xcode project (see docs/IOS.md for instructions)");
                        }
                    }

                    // Create iOS project structure template
                    let ios_project = output.join("ios");
                    fs::create_dir_all(&ios_project)?;

                    // Create a simple build script for macOS
                    let build_script = r#"#!/bin/bash
# iOS build script for macOS
# Run this on macOS with Xcode installed

set -e

echo "Building Kiln iOS static library..."

# Build for device
cargo build --release --target aarch64-apple-ios -p kiln-player

# Build for simulator (Apple Silicon)
cargo build --release --target aarch64-apple-ios-sim -p kiln-player

echo "Static libraries built"
echo "Next: Create Xcode project and integrate with UIKit"
echo "See docs/IOS.md for complete instructions"
"#;

                    fs::write(ios_project.join("build.sh"), build_script)?;

                    println!("iOS project structure created: {}", ios_project.display());
                    println!("Build script: {}", ios_project.join("build.sh").display());
                    println!("On macOS, run: cd {} && ./build.sh", ios_project.display());
                    println!(
                        "Then follow docs/IOS.md to create Xcode project and sign for App Store"
                    );
                }
                _ => {
                    anyhow::bail!(
                        "Unknown platform: {}. Supported: web, windows, macos, linux, android, ios",
                        platform
                    );
                }
            }
        }
    }

    Ok(())
}
