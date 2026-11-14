use actix_web::{HttpResponse, web};
use generator::{inputs::InvalidSettings, metadata::create_metadata};
use logic_catalog::LogicCatalog;
use preset_catalog::PresetCatalog;
use rand::{SeedableRng, rngs::StdRng};
use retiling_catalog::RetilingCatalog;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use server::store::Store;
use tokio::sync::Semaphore;

pub struct GenerationState {
    pub logic: LogicCatalog,
    pub retiling: RetilingCatalog,
    pub presets: PresetCatalog,
    pub map_pool: Vec<u8>,
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
        let settings = worker_state.presets.resolve_settings(&request.settings)?;
        let notices = settings.notices.clone();
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
            &worker_state.retiling,
            "Desert",
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
