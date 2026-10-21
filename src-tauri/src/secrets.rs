use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::sync::Mutex;

/// The COS key pair, kept in the OS credential store rather than in `.settings.dat`.
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct CosCredentials {
    pub secret_id: String,
    pub secret_key: String,
}

pub trait SecretStore: Send + Sync {
    fn get(&self) -> Result<Option<String>, String>;
    fn set(&self, value: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
}

/// macOS Keychain, Windows Credential Manager or the Secret Service on Linux.
pub struct Keychain {
    service: String,
    account: &'static str,
}

impl Keychain {
    /// Entries are namespaced by the app identifier, so dev builds don't share production keys.
    pub fn new(identifier: &str) -> Self {
        Keychain { service: identifier.to_string(), account: "cos-credentials" }
    }

    fn entry(&self) -> Result<keyring::Entry, String> {
        keyring::Entry::new(&self.service, self.account).map_err(|e| format!("System keychain unavailable: {e}"))
    }
}

impl SecretStore for Keychain {
    fn get(&self) -> Result<Option<String>, String> {
        match self.entry()?.get_password() {
            Ok(value) => Ok(Some(value)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Failed to read from the system keychain: {e}")),
        }
    }

    fn set(&self, value: &str) -> Result<(), String> {
        self.entry()?
            .set_password(value)
            .map_err(|e| format!("Failed to save to the system keychain: {e}"))
    }

    fn delete(&self) -> Result<(), String> {
        match self.entry()?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Failed to remove from the system keychain: {e}")),
        }
    }
}

pub struct Secrets(pub Box<dyn SecretStore>);

impl Secrets {
    pub fn credentials(&self) -> Result<CosCredentials, String> {
        match self.0.get()? {
            Some(json) => serde_json::from_str(&json).map_err(|e| format!("Unreadable COS credentials: {e}")),
            None => Ok(CosCredentials::default()),
        }
    }

    pub fn set_credentials(&self, credentials: &CosCredentials) -> Result<(), String> {
        let json = serde_json::to_string(credentials).map_err(|e| e.to_string())?;
        self.0.set(&json)?;
        // Only report success once the value can be read back, since callers may then
        // delete the only other copy
        if self.credentials()? != *credentials {
            return Err("The system keychain did not keep the COS credentials".to_string());
        }
        Ok(())
    }

    pub fn clear(&self) -> Result<(), String> {
        self.0.delete()
    }
}

#[cfg(test)]
#[derive(Default)]
pub struct MemoryStore {
    pub value: Mutex<Option<String>>,
    pub fail_writes: bool,
}

#[cfg(test)]
impl SecretStore for std::sync::Arc<MemoryStore> {
    fn get(&self) -> Result<Option<String>, String> {
        Ok(self.value.lock().unwrap().clone())
    }

    fn set(&self, value: &str) -> Result<(), String> {
        if self.fail_writes {
            return Err("keychain locked".to_string());
        }
        *self.value.lock().unwrap() = Some(value.to_string());
        Ok(())
    }

    fn delete(&self) -> Result<(), String> {
        *self.value.lock().unwrap() = None;
        Ok(())
    }
}
