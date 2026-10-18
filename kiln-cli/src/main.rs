use anyhow::Result;
use clap::{Parser, Subcommand};
use kiln_assets::{AssetCache, Archive};
use std::path::PathBuf;

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
    }

    Ok(())
}
