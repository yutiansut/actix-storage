use std::sync::Arc;

#[cfg(feature = "v01-compat")]
use std::ops::Deref;

use actix_storage::{dev::Store, Result as StorageResult, StorageError};
use sled::Tree;

use crate::{SledConfig, SledError};

/// A simple implementation of [`Store`](actix_storage::dev::Store) based on Sled
///
/// This provider doesn't support key expiration thus Storage will return errors when trying to use methods
/// that require expiration functionality if there is no expiry provided.
///
/// ## Example
/// ```no_run
/// use actix_storage::Storage;
/// use actix_storage_sled::SledStore;
/// use actix_web::{App, HttpServer};
///
/// #[actix_web::main]
/// async fn main() -> std::io::Result<()> {
///     let db = SledStore::new().expect("Error opening the database");
///     let storage = Storage::build().store(db).finish();
///     let server = HttpServer::new(move || {
///         App::new()
///             .data(storage.clone())
///     });
///     server.bind("localhost:5000")?.run().await
/// }
/// ```
#[derive(Debug)]
pub struct SledStore {
    db: sled::Db,
}

impl SledStore {
    pub fn new() -> Result<Self, SledError> {
        Ok(Self {
            db: SledConfig::default().open()?,
        })
    }

    pub fn from_db(db: sled::Db) -> Self {
        Self { db }
    }

    #[cfg(not(feature = "v01-compat"))]
    fn get_tree(&self, scope: Arc<[u8]>) -> StorageResult<Tree> {
        self.db.open_tree(scope).map_err(StorageError::custom)
    }

    #[cfg(feature = "v01-compat")]
    fn get_tree(&self, scope: Arc<[u8]>) -> StorageResult<Tree> {
        if scope.as_ref() == &actix_storage::GLOBAL_SCOPE {
            Ok(self.db.deref().clone())
        } else {
            self.db.open_tree(scope).map_err(StorageError::custom)
        }
    }
}

#[async_trait::async_trait]
impl Store for SledStore {
    async fn set(&self, scope: Arc<[u8]>, key: Arc<[u8]>, value: Arc<[u8]>) -> StorageResult<()> {
        match self.get_tree(scope)?.insert(key, value.as_ref()) {
            Ok(_) => Ok(()),
            Err(err) => Err(StorageError::custom(err)),
        }
    }

    async fn get(&self, scope: Arc<[u8]>, key: Arc<[u8]>) -> StorageResult<Option<Arc<[u8]>>> {
        Ok(self
            .get_tree(scope)?
            .get(key)
            .map_err(StorageError::custom)?
            .map(|val| val.as_ref().into()))
    }

    async fn delete(&self, scope: Arc<[u8]>, key: Arc<[u8]>) -> StorageResult<()> {
        match self.get_tree(scope)?.remove(key) {
            Ok(_) => Ok(()),
            Err(err) => Err(StorageError::custom(err)),
        }
    }

    async fn contains_key(&self, scope: Arc<[u8]>, key: Arc<[u8]>) -> StorageResult<bool> {
        match self.get_tree(scope)?.contains_key(key) {
            Ok(res) => Ok(res),
            Err(err) => Err(StorageError::custom(err)),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use actix_storage::tests::*;
    use std::time::Duration;

    async fn open_database() -> sled::Db {
        let mut tries: u8 = 0;
        loop {
            tries += 1;
            let db = SledConfig::default().temporary(true).open();
            match db {
                Ok(db) => return db,
                Err(err) => {
                    if tries > 10 {
                        panic!("{}", err)
                    };
                }
            }
            actix::clock::sleep(Duration::from_millis(500)).await;
        }
    }

    #[test]
    fn test_sled_basic_store() {
        test_store(Box::pin(async {
            SledStore::from_db(open_database().await)
        }));
    }

    #[test]
    fn test_sled_basic_formats() {
        impl Clone for SledStore {
            fn clone(&self) -> Self {
                Self {
                    db: self.db.clone(),
                }
            }
        }
        test_all_formats(Box::pin(async {
            SledStore::from_db(open_database().await)
        }));
    }
}
