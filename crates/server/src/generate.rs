use actix_web::{HttpResponse, web};
use generator::{inputs::InvalidSettings, metadata::create_metadata};
use logic_catalog::LogicCatalog;
use preset_catalog::PresetCatalog;
use rand::{SeedableRng, rngs::StdRng};
use seed_data::{SettingsErrors, SettingsMode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use server::store::Store;
use tokio::sync::Semaphore;

pub struct GenerationState {
    pub logic: LogicCatalog,
    pub presets: PresetCatalog,
    pub map_pool: Vec<u8>,
    pub palette_profiles: retiling_catalog::palette::PaletteProfiles,
    pub logic_hash: String,
    pub retiling_hash: String,
    pub store: Store,
    pub slot: std::sync::Arc<Semaphore>,
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub settings: Value,
}

#[derive(Serialize)]
struct GenerateResponse {
    seed: String,
    notices: Vec<String>,
}

pub async fn generate_seed(
    state: web::Data<GenerationState>,
    request: web::Json<GenerateRequest>,
) -> HttpResponse {
    // Wait without occupying an HTTP or blocking-pool worker; all Actix workers share this slot.
    let permit = state.slot.clone().acquire_owned().await.unwrap();
    let request = request.into_inner();
    let worker_state = state.clone();
    let result = web::block(move || -> anyhow::Result<_> {
        // A started blocking task keeps the slot even if its HTTP handler is dropped.
        let permit = permit;
        let report = worker_state
            .presets
            .process_settings(&request.settings, SettingsMode::Strict)?;
        let mut notices = Vec::new();
        for notice in &report.notices {
            notices.push(notice.to_string());
        }
        let settings = report.finish_processing()?;
        let numeric_seed = rand::random();
        let metadata = create_metadata(
            numeric_seed,
            &worker_state.logic_hash,
            &worker_state.retiling_hash,
        )?;
        let mut rng = StdRng::seed_from_u64(numeric_seed);
        let seed = generator::generate(
            settings,
            &worker_state.logic,
            &worker_state.palette_profiles,
            "desert",
            Some(&worker_state.map_pool),
            metadata,
            &mut rng,
        )?;
        let mut bytes = Vec::new();
        seed.write(&mut bytes)?;
        Ok((seed.metadata.seed_name, bytes, notices, permit))
    })
    .await;
    let (seed, bytes, notices, _permit) = match result {
        Ok(Ok(result)) => result,
        Ok(Err(error)) if error.is::<SettingsErrors>() => {
            return HttpResponse::UnprocessableEntity().json(json!({
                "error": error.to_string(),
                "errors": error.downcast_ref::<SettingsErrors>().unwrap().0,
            }));
        }
        Ok(Err(error)) if error.is::<InvalidSettings>() => {
            return HttpResponse::UnprocessableEntity().json(json!({ "error": error.to_string() }));
        }
        error => {
            eprintln!("Generating seed: {error:?}");
            return HttpResponse::InternalServerError()
                .json(json!({ "error": "Seed generation failed." }));
        }
    };
    if let Err(error) = state
        .store
        .save_object(&format!("seeds/{seed}.json"), bytes)
        .await
    {
        eprintln!("Saving seed {seed}: {error:#}");
        return HttpResponse::InternalServerError()
            .json(json!({ "error": "Could not save the generated seed." }));
    }
    HttpResponse::Created().json(GenerateResponse { seed, notices })
}

pub async fn get_seed(state: web::Data<GenerationState>, name: web::Path<String>) -> HttpResponse {
    match state.store.load_object(&format!("seeds/{name}.json")).await {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/json")
            .body(bytes),
        Err(error) => match error.downcast_ref::<object_store::Error>() {
            Some(object_store::Error::NotFound { .. }) => {
                HttpResponse::NotFound().json(json!({ "error": "Seed not found." }))
            }
            _ => {
                eprintln!("Loading seed {name}: {error:#}");
                HttpResponse::InternalServerError()
                    .json(json!({ "error": "Could not load the seed." }))
            }
        },
    }
}
