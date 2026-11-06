mod export;
use anyhow::Result;
use clap::{Parser, Subcommand};
use kiln_assets::{Archive, AssetCache};
use kiln_updater::{dlc::DlcMetadata, generate_patch};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// Export a saved project with a matching prebuilt native runtime.
    Export {
        #[arg(short, long, default_value = std::env::consts::OS)]
        platform: String,
        /// Project manifest (TOML or RON), independent of the current directory.
        #[arg(long, default_value = "kiln.toml")]
        project: PathBuf,
        /// New/empty output directory. Existing nonempty exports are preserved.
        #[arg(short, long, default_value = "export/game")]
        output: PathBuf,
        /// Prebuilt player (defaults to kiln-player beside this exporter).
        #[arg(long)]
        player: Option<PathBuf>,
        /// Correlate an editor request with its output receipt.
        #[arg(long)]
        job_id: Option<String>,
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
    /// Import a TTF font and build an SDF atlas (requires --features fonts)
    ImportFont {
        /// Input TTF font file
        #[arg(short, long)]
        input: PathBuf,
        /// Output atlas PNG path
        #[arg(short, long, default_value = "font_atlas.png")]
        output: PathBuf,
        /// Output font metadata RON path
        #[arg(short, long, default_value = "font.ron")]
        metadata: PathBuf,
        /// Font size in pixels
        #[arg(short, long, default_value = "32")]
        size: f32,
        /// Atlas width in pixels
        #[arg(long, default_value = "512")]
        width: u32,
        /// Atlas height in pixels
        #[arg(long, default_value = "512")]
        height: u32,
        /// Characters to include (default: ASCII printable + space)
        #[arg(
            long,
            default_value = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789 .,!?:;'\"-()[]{}"
        )]
        chars: String,
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
        Commands::ImportFont {
            input,
            output,
            metadata,
            size,
            width,
            height,
            chars,
        } => {
            let ttf_data = std::fs::read(&input)
                .map_err(|e| anyhow::anyhow!("Failed to read TTF file '{:?}': {e}", input))?;

            let config = kiln_ui::sdf_builder::SdfBuilderConfig {
                font_size: size,
                atlas_width: width,
                atlas_height: height,
                padding: 2,
                chars: chars.chars().collect(),
                spread: 8,
            };

            let result = kiln_ui::sdf_builder::build_sdf_atlas(&ttf_data, &config)
                .map_err(|e| anyhow::anyhow!("SDF atlas build failed: {e}"))?;

            // Save atlas PNG.
            use std::io::Cursor;
            let mut buf = Vec::new();
            result
                .atlas
                .write_to(&mut Cursor::new(&mut buf), image::ImageFormat::Png)
                .map_err(|e| anyhow::anyhow!("Failed to encode PNG: {e}"))?;
            std::fs::write(&output, &buf)
                .map_err(|e| anyhow::anyhow!("Failed to write atlas '{:?}': {e}", output))?;
            println!("SDF atlas written to: {}", output.display());

            // Save font metadata RON.
            let ron_str = ron::to_string(&result.font)
                .map_err(|e| anyhow::anyhow!("Failed to serialize font metadata: {e}"))?;
            std::fs::write(&metadata, ron_str)
                .map_err(|e| anyhow::anyhow!("Failed to write metadata '{:?}': {e}", metadata))?;
            println!("Font metadata written to: {}", metadata.display());
        }
        Commands::Export {
            platform,
            project,
            output,
            player,
            job_id,
        } => {
            export::run(export::Options {
                platform,
                project,
                output,
                player,
                job_id,
            })?;
        }
    }

    Ok(())
}
