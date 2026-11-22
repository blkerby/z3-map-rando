use anyhow::{Result, bail};
use object_store::{
    ObjectStore, ObjectStoreExt, PutMode, PutOptions, aws::AmazonS3Builder, local::LocalFileSystem,
    memory::InMemory, path::Path, prefix::PrefixStore,
};
use std::{io::Cursor, sync::Arc};

/// Whole-object storage with logical keys and transparently Zstd-compressed physical objects.
pub struct Store {
    inner: Arc<dyn ObjectStore>,
    check_before_put: bool,
}

impl Store {
    pub fn open_store(spec: &str, check_before_put: bool) -> Result<Self> {
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
        Ok(Self {
            inner,
            check_before_put,
        })
    }

    pub async fn save_object(&self, key: &str, bytes: Vec<u8>) -> Result<()> {
        let path = Path::parse(format!("{key}.zst"))?;
        let compressed =
            actix_web::web::block(move || zstd::stream::encode_all(Cursor::new(bytes), 3))
                .await??;
        let mode = if self.check_before_put {
            // shortcut: a concurrent writer can claim the key after HEAD; use atomic create if supported.
            match self.inner.head(&path).await {
                Ok(_) => bail!("Object {path} already exists"),
                Err(object_store::Error::NotFound { .. }) => PutMode::Overwrite,
                Err(error) => return Err(error.into()),
            }
        } else {
            PutMode::Create
        };
        self.inner
            .put_opts(
                &path,
                compressed.into(),
                PutOptions {
                    mode,
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
