use actix_files::Files;
use actix_web::{App, HttpServer};
use anyhow::{Context, Result};
use clap::Parser;
use std::{net::SocketAddr, path::PathBuf};

#[derive(Parser)]
#[command(name = "z3-map-rando-server", about = "Serve the randomizer website")]
struct Args {
    /// Address and port to listen on.
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
    /// Directory containing the built frontend.
    #[arg(long, default_value = "frontend/build")]
    static_dir: PathBuf,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    std::fs::metadata(args.static_dir.join("index.html")).with_context(|| {
        format!(
            "reading {}/index.html; run npm run build in frontend/ first",
            args.static_dir.display()
        )
    })?;

    let static_dir = args.static_dir.clone();
    let server = HttpServer::new(move || {
        App::new().service(
            Files::new("/", &static_dir)
                .index_file("index.html")
                .redirect_to_slash_directory(),
        )
    })
    .bind(args.bind)?;

    eprintln!(
        "Serving {} at http://{}",
        args.static_dir.display(),
        args.bind
    );
    server.run().await?;
    Ok(())
}
