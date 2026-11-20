use actix_files::{Files, NamedFile};
use actix_web::middleware::Logger;
use actix_web::{App, HttpServer, web};
use anyhow::{Context, Result};
use clap::Parser;
use generate::GenerationState;
use generator::metadata::compute_catalog_hash;
use server::store::Store;
use std::{fs, net::SocketAddr, path::PathBuf, sync::Arc};
use tokio::sync::Semaphore;

mod generate;
mod settings;

#[derive(Parser)]
#[command(name = "z3-map-rando-server", about = "Serve the randomizer website")]
struct Args {
    /// Address and port to listen on.
    #[arg(long, default_value = "127.0.0.1:8080")]
    bind: SocketAddr,
    /// Directory containing the built frontend.
    #[arg(long, default_value = "frontend/build")]
    static_dir: PathBuf,
    /// Object store: mem, file:<directory>, or s3://<bucket>/<prefix>.
    #[arg(long, default_value = "mem")]
    store: String,
    /// Serve only the API, not the built frontend (for the Vite dev server, which serves the pages).
    #[arg(long)]
    api_only: bool,
}

#[actix_web::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .init();
    if !args.api_only {
        std::fs::metadata(args.static_dir.join("index.html")).with_context(|| {
            format!(
                "reading {}/index.html; run npm run build in frontend/ first",
                args.static_dir.display()
            )
        })?;
    }

    let config_path = std::path::Path::new("catalog-build.toml");
    let config: toml::Table = if config_path.exists() {
        toml::from_str(&fs::read_to_string(config_path)?)?
    } else {
        toml::Table::new()
    };
    let catalog_dir = PathBuf::from(
        config
            .get("output_directory")
            .and_then(toml::Value::as_str)
            .unwrap_or("build"),
    );
    let map_pool_path = PathBuf::from(
        config
            .get("map_pool")
            .and_then(toml::Value::as_str)
            .unwrap_or("maps/map_pool.bin"),
    );
    let logic_bytes = fs::read(catalog_dir.join("logic_catalog.bin"))
        .context("reading logic catalog; run catalog_builder first")?;
    let retiling_bytes = fs::read(catalog_dir.join("retiling_catalog.bin"))
        .context("reading retiling catalog; run catalog_builder first")?;
    let patches =
        patch_catalog::decode_catalog(fs::File::open(catalog_dir.join("patch_catalog.bin"))?)
            .context("decoding patch catalog")?;
    let state = web::Data::new(GenerationState {
        version: fs::read_to_string(catalog_dir.join("version.txt"))
            .context("reading version; run catalog_builder first")?,
        logic: logic_catalog::decode_catalog(logic_bytes.as_slice())?,
        presets: preset_catalog::decode_catalog(fs::File::open(
            catalog_dir.join("preset_catalog.bin"),
        )?)?,
        map_pool: fs::read(&map_pool_path)
            .with_context(|| format!("reading map pool {}", map_pool_path.display()))?,
        logic_hash: compute_catalog_hash(&logic_bytes),
        retiling: retiling_catalog::decode_catalog(retiling_bytes.as_slice())?,
        patches,
        retiling_hash: compute_catalog_hash(&retiling_bytes),
        store: Store::open_store(&args.store)?,
        slot: Arc::new(Semaphore::new(1)),
    });

    let static_dir = args.static_dir.clone();
    let api_only = args.api_only;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(state.clone())
            .route("/api/generate", web::post().to(generate::generate_seed))
            .route(
                "/api/settings/process",
                web::post().to(settings::process_settings),
            )
            .route("/api/seed/{name}", web::get().to(generate::get_seed))
            .route(
                "/api/seed/{name}/retiling",
                web::get().to(generate::get_seed_retiling),
            )
            .configure(|config| {
                if api_only {
                    return;
                }
                config
                    .route(
                        "/seed/{name}/",
                        web::get().to({
                            // Seed pages aren't prerendered; the frontend's SPA fallback renders them.
                            let fallback = static_dir.join("spa.html");
                            move || NamedFile::open_async(fallback.clone())
                        }),
                    )
                    .service(
                        Files::new("/", &static_dir)
                            .index_file("index.html")
                            .redirect_to_slash_directory(),
                    );
            })
    })
    .bind(args.bind)?;

    if api_only {
        log::info!("Serving the API at http://{}", args.bind);
    } else {
        log::info!(
            "Serving {} at http://{}",
            args.static_dir.display(),
            args.bind
        );
    }
    server.run().await?;
    Ok(())
}
