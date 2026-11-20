use actix_web::{HttpResponse, web};
use generator::{inputs::InvalidSettings, metadata::create_metadata};
use logic_catalog::LogicCatalog;
use preset_catalog::PresetCatalog;
use rand::{SeedableRng, rngs::StdRng};
use seed_data::{SeedData, SettingsErrors, SettingsMode};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use server::store::Store;
use tokio::sync::Semaphore;

pub struct GenerationState {
    /// Project version, from the catalog builder's `version.txt`.
    pub version: String,
    pub logic: LogicCatalog,
    pub presets: PresetCatalog,
    pub map_pool: Vec<u8>,
    pub retiling: retiling_catalog::RetilingCatalog,
    pub patches: patch_catalog::PatchCatalog,
    pub logic_hash: String,
    pub retiling_hash: String,
    pub store: Store,
    pub slot: std::sync::Arc<Semaphore>,
}

#[derive(Deserialize)]
pub struct GenerateRequest {
    pub settings: Value,
    /// Settings revision of the requesting frontend's bundled data.
    pub revision: String,
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
    // A frontend built against other settings data may have edited settings this server reads
    // differently; the player reloads the page to get the matching frontend.
    if request.revision != state.presets.revision {
        return HttpResponse::Conflict().json(json!({
            "error": "The randomizer has been updated. Reload the page to continue.",
        }));
    }
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
            &worker_state.version,
            numeric_seed,
            &worker_state.logic_hash,
            &worker_state.retiling_hash,
        )?;
        let mut rng = StdRng::seed_from_u64(numeric_seed);
        let seed = generator::generate(
            settings,
            &worker_state.logic,
            &worker_state.retiling.palette_profiles,
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
            log::error!("Generating seed: {error:?}");
            return HttpResponse::InternalServerError()
                .json(json!({ "error": "Seed generation failed." }));
        }
    };
    if let Err(error) = state
        .store
        .save_object(&format!("seeds/{seed}.json"), bytes)
        .await
    {
        log::error!("Saving seed {seed}: {error:#}");
        return HttpResponse::InternalServerError()
            .json(json!({ "error": "Could not save the generated seed." }));
    }
    HttpResponse::Created().json(GenerateResponse { seed, notices })
}

pub async fn get_seed(state: web::Data<GenerationState>, name: web::Path<String>) -> HttpResponse {
    match load_seed(&state, &name).await {
        Ok(bytes) => HttpResponse::Ok()
            .content_type("application/json")
            .body(bytes),
        Err(response) => response,
    }
}

pub async fn get_seed_retiling(
    state: web::Data<GenerationState>,
    name: web::Path<String>,
) -> HttpResponse {
    let bytes = match load_seed(&state, &name).await {
        Ok(bytes) => bytes,
        Err(response) => return response,
    };
    let result = web::block(move || -> anyhow::Result<Vec<u8>> {
        let seed = SeedData::read(bytes.as_slice())?;
        let catalog = patcher::filter_retiling_catalog(&seed, &state.retiling, &state.patches)?;
        let mut output = Vec::new();
        retiling_catalog::encode_catalog(&catalog, &mut output, 3)?;
        Ok(output)
    })
    .await;
    match result {
        Ok(Ok(bytes)) => HttpResponse::Ok()
            .content_type("application/octet-stream")
            .body(bytes),
        error => {
            log::error!("Preparing retiling catalog for seed {name}: {error:?}");
            HttpResponse::InternalServerError()
                .json(json!({ "error": "Could not prepare the retiling catalog." }))
        }
    }
}

async fn load_seed(state: &GenerationState, name: &str) -> Result<Vec<u8>, HttpResponse> {
    match state.store.load_object(&format!("seeds/{name}.json")).await {
        Ok(bytes) => Ok(bytes),
        Err(error) => match error.downcast_ref::<object_store::Error>() {
            Some(object_store::Error::NotFound { .. }) => {
                Err(HttpResponse::NotFound().json(json!({ "error": "Seed not found." })))
            }
            _ => {
                log::error!("Loading seed {name}: {error:#}");
                Err(HttpResponse::InternalServerError()
                    .json(json!({ "error": "Could not load the seed." })))
            }
        },
    }
}
