use anyhow::Result;
use chrono::{DateTime, SecondsFormat, Utc};
use rand::{Rng, SeedableRng, rngs::StdRng};
use seed_data::SeedMetadata;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;
use std::time::{SystemTime, UNIX_EPOCH};

pub fn create_metadata(
    version: &str,
    numeric_seed: u64,
    logic_hash: &str,
    retiling_hash: &str,
) -> Result<SeedMetadata> {
    let now = SystemTime::now();
    let timestamp = now.duration_since(UNIX_EPOCH)?.as_nanos();
    let mut naming_seed = [0; 32];
    naming_seed[..8].copy_from_slice(&numeric_seed.to_le_bytes());
    naming_seed[8..24].copy_from_slice(&timestamp.to_le_bytes());
    let mut rng = StdRng::from_seed(naming_seed);
    // Omit vowels and characters resembling them to reduce the chance of words.
    let alphabet = b"256789BCDFGHJKLMNPQRSTVWXYZbcdfghjkmnpqrstvwxyz";
    let mut seed_name = String::new();
    for _ in 0..9 {
        seed_name.push(alphabet[rng.random_range(0..alphabet.len())] as char);
    }
    Ok(SeedMetadata {
        seed_name,
        generator_version: version.to_owned(),
        generator_commit: None,
        logic_catalog_hash: logic_hash.to_owned(),
        retiling_catalog_hash: retiling_hash.to_owned(),
        rng_seed: numeric_seed.to_string(),
        generated_at: DateTime::<Utc>::from(now).to_rfc3339_opts(SecondsFormat::Nanos, true),
    })
}

pub fn compute_catalog_hash(bytes: &[u8]) -> String {
    let mut hash = String::new();
    for byte in Sha256::digest(bytes) {
        write!(&mut hash, "{byte:02x}").unwrap();
    }
    hash
}
