use crate::generate::GenerationState;
use actix_web::{HttpResponse, web};
use seed_data::{SettingsDocument, SettingsIssue, SettingsMode};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SettingsKind {
    Generation,
    Customization,
}

#[derive(Deserialize)]
pub struct ProcessRequest {
    pub kind: SettingsKind,
    pub document: Value,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum ProcessResult {
    /// The cleaned document, what recovery changed, and the cross-setting findings left for the
    /// player to fix.
    Processed {
        document: SettingsDocument,
        notices: Vec<SettingsIssue>,
        errors: Vec<SettingsIssue>,
    },
    /// Recovery failed, which indicates a bug; the player can only discard the document.
    Unrecoverable,
}

#[derive(Serialize)]
struct ProcessResponse {
    /// The settings revision the document was processed under.
    revision: String,
    #[serde(flatten)]
    result: ProcessResult,
}

/// Process one stored or imported settings document in recovery mode.
///
/// A failure caused by the input, including a panic, is reported as unrecoverable rather than as a
/// server error, so the browser can tell it apart from a server that is unavailable.
pub async fn process_settings(
    state: web::Data<GenerationState>,
    request: web::Json<ProcessRequest>,
) -> HttpResponse {
    let presets = &state.presets;
    let processed = catch_unwind(AssertUnwindSafe(|| match request.kind {
        SettingsKind::Generation => presets
            .process_settings(&request.document, SettingsMode::Recover)
            .map(|report| (report.document, report.notices, report.errors)),
        SettingsKind::Customization => presets
            .process_customization(&request.document, SettingsMode::Recover)
            .map(|report| (report.document, report.notices, report.errors)),
    }));
    let result = match processed {
        Ok(Ok((document, notices, errors))) => ProcessResult::Processed {
            document,
            notices,
            errors,
        },
        Ok(Err(error)) => {
            log::error!("Processing settings: {error:#}");
            ProcessResult::Unrecoverable
        }
        Err(_) => {
            log::error!("Processing settings panicked");
            ProcessResult::Unrecoverable
        }
    };
    HttpResponse::Ok().json(ProcessResponse {
        revision: presets.revision.clone(),
        result,
    })
}
