use anyhow::{Context as _, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;
use tashi_vertex::KeySecret;

#[derive(Parser, Debug)]
struct Args {
    /// Output directory to write node{i}.secret and node{i}.public
    #[arg(long, default_value = "/keys")]
    out_dir: PathBuf,

    /// Number of nodes to generate keys for
    #[arg(long, default_value_t = 4)]
    count: usize,

    /// If set, do not overwrite existing files; exit successfully if they exist
    #[arg(long, default_value_t = true)]
    idempotent: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create out dir {:?}", args.out_dir))?;

    // If idempotent, and all files exist, exit success.
    if args.idempotent {
        let mut all_exist = true;
        for i in 1..=args.count {
            let s = args.out_dir.join(format!("node{i}.secret"));
            let p = args.out_dir.join(format!("node{i}.public"));
            all_exist &= s.exists() && p.exists();
        }
        if all_exist {
            println!("keys already exist in {:?}; nothing to do", args.out_dir);
            return Ok(());
        }
    }

    for i in 1..=args.count {
        let secret = KeySecret::generate();
        let public = secret.public();

        let secret_path = args.out_dir.join(format!("node{i}.secret"));
        let public_path = args.out_dir.join(format!("node{i}.public"));

        fs::write(&secret_path, secret.to_string() + "\n")
            .with_context(|| format!("write {:?}", secret_path))?;
        fs::write(&public_path, public.to_string() + "\n")
            .with_context(|| format!("write {:?}", public_path))?;

        println!("generated node{i}:");
        println!("  secret: {}", secret_path.display());
        println!("  public: {}", public_path.display());
    }

    Ok(())
}
