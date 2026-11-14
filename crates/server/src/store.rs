use anyhow::{Result, bail};
use object_store::{
    ObjectStore, ObjectStoreExt, PutMode, PutOptions, aws::AmazonS3Builder, local::LocalFileSystem,
    memory::InMemory, path::Path, prefix::PrefixStore,
};
use std::{io::Cursor, sync::Arc};

/// Whole-object storage with logical keys and transparently Zstd-compressed physical objects.
pub struct Store {
    inner: Arc<dyn ObjectStore>,
}

impl Store {
    pub fn open_store(spec: &str) -> Result<Self> {
        let inner: Arc<dyn ObjectStore> = if spec == "mem" {
            Arc::new(InMemory::new())
        } else if let Some(directory) = spec.strip_prefix("file:") {
            anyhow::ensure!(!directory.is_empty(), "file store requires a directory");
            std::fs::create_dir_all(directory)?;
            Arc::new(LocalFileSystem::new_with_prefix(directory)?)
        } else if spec.starts_with("s3://") {
            let (_, prefix) = object_store::ObjectStoreScheme::parse(&url::Url::parse(spec)?)?;
            let store = AmazonS3Builder::from_env().with_url(spec).build()?;
            Arc::new(PrefixStore::new(store, prefix))
        } else {
            bail!("store must be mem, file:<directory>, or s3://<bucket>/<prefix>");
        };
        Ok(Self { inner })
    }

    pub async fn save_object(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        let compressed =
            actix_web::web::block(move || zstd::stream::encode_all(Cursor::new(bytes), 3))
                .await??;
        self.inner
            .put_opts(
                &Path::parse(format!("{key}.zst"))?,
                compressed.into(),
                PutOptions {
                    mode: PutMode::Create,
                    ..Default::default()
                },
            )
            .await?;
        Ok(())
    }

    pub async fn load_object(&self, key: &str) -> Result<Vec<u8>> {
        let bytes = self
            .inner
            .get(&Path::parse(format!("{key}.zst"))?)
            .await?
            .bytes()
            .await?;
        Ok(actix_web::web::block(move || zstd::stream::decode_all(Cursor::new(bytes))).await??)
    }
}
