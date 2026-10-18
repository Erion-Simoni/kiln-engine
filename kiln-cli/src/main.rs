use anyhow::Result;
use clap::{Parser, Subcommand};
use kiln_assets::{AssetCache, Archive};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

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
    },
    /// Verify archive integrity
    Verify {
        /// Archive to verify
        #[arg(short, long)]
        archive: PathBuf,
    },
    /// Export project for web (WASM build + HTML shell)
    Export {
        /// Output directory for web build
        #[arg(short, long, default_value = "web-export")]
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
        Commands::Pack { source, output } => {
            let files: Vec<PathBuf> = std::fs::read_dir(&source)?
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .map(|e| e.path())
                .collect();

            println!("Packing {} files into: {}", files.len(), output.display());
            let mut archive = Archive::new(output);
            archive.pack(&files)?;
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
        }
        Commands::Export { output } => {
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
    }

    Ok(())
}
