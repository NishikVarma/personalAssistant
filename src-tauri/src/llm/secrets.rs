use std::sync::Mutex;

use crate::error::{AppError, AppResult};

/// Abstraction over OS secure credential storage so the rest of the app never
/// touches `keyring` directly and tests can use an in-memory store.
pub trait SecretStore: Send + Sync {
    fn get(&self, service: &str, account: &str) -> AppResult<Option<String>>;
    fn set(&self, service: &str, account: &str, value: &str) -> AppResult<()>;
    fn delete(&self, service: &str, account: &str) -> AppResult<bool>;
}

/// OS keyring-backed store (Windows Credential Manager / freedesktop Secret Service).
pub struct KeyringStore;

impl KeyringStore {
    pub fn new() -> Self {
        Self
    }

    fn entry(&self, service: &str, account: &str) -> AppResult<keyring::Entry> {
        keyring::Entry::new(service, account).map_err(|e| secret_error("create entry", e))
    }
}

impl Default for KeyringStore {
    fn default() -> Self {
        Self::new()
    }
}

fn secret_error(action: &str, err: keyring::Error) -> AppError {
    match err {
        keyring::Error::NoEntry => AppError::NotFound("secret not found".to_string()),
        other => AppError::Secret(format!("{action}: {other}")),
    }
}

impl SecretStore for KeyringStore {
    fn get(&self, service: &str, account: &str) -> AppResult<Option<String>> {
        match self.entry(service, account)?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(secret_error("read secret", e)),
        }
    }

    fn set(&self, service: &str, account: &str, value: &str) -> AppResult<()> {
        self.entry(service, account)?
            .set_password(value)
            .map_err(|e| secret_error("store secret", e))
    }

    fn delete(&self, service: &str, account: &str) -> AppResult<bool> {
        match self.entry(service, account)?.delete_credential() {
            Ok(()) => Ok(true),
            Err(keyring::Error::NoEntry) => Ok(false),
            Err(e) => Err(secret_error("delete secret", e)),
        }
    }
}

/// In-memory store for tests and environments without a keyring service.
#[derive(Default)]
pub struct MemoryStore {
    entries: Mutex<std::collections::HashMap<(String, String), String>>,
}

impl MemoryStore {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecretStore for MemoryStore {
    fn get(&self, service: &str, account: &str) -> AppResult<Option<String>> {
        let map = self.entries.lock().expect("secret map lock");
        Ok(map.get(&(service.to_string(), account.to_string())).cloned())
    }

    fn set(&self, service: &str, account: &str, value: &str) -> AppResult<()> {
        let mut map = self.entries.lock().expect("secret map lock");
        map.insert(
            (service.to_string(), account.to_string()),
            value.to_string(),
        );
        Ok(())
    }

    fn delete(&self, service: &str, account: &str) -> AppResult<bool> {
        let mut map = self.entries.lock().expect("secret map lock");
        Ok(map
            .remove(&(service.to_string(), account.to_string()))
            .is_some())
    }
}
