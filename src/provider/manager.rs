use std::collections::HashMap;
use std::sync::Arc;

use super::Provider;

#[derive(Default)]
pub struct ProviderManager {
    providers: HashMap<String, Arc<Provider>>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, provider: Provider) {
        let key = provider.key.clone();
        self.providers.insert(key, Arc::new(provider));
    }

    pub fn get_for_path(&self, path: &str) -> Option<&Arc<Provider>> {
        self.providers
            .values()
            .find(|p| p.compatibility.is_compatible(path))
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Arc<Provider>)> {
        self.providers.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provider::compatibility::Compatibility;
    use crate::test_utils::make_provider;

    #[test]
    fn returns_matching_provider() {
        let mut mgr = ProviderManager::new();

        let mut openai_compat = Compatibility::default();
        openai_compat.openai_chat = true;
        mgr.add(make_provider(
            "openai",
            "https://example.com",
            openai_compat,
        ));

        let mut anthropic_compat = Compatibility::default();
        anthropic_compat.anthropic_messages = true;
        mgr.add(make_provider(
            "anthropic",
            "https://example.com",
            anthropic_compat,
        ));

        let openai = mgr.get_for_path("/v1/chat/completions").unwrap();
        assert_eq!(openai.name, "openai");

        let anthropic = mgr.get_for_path("/v1/messages").unwrap();
        assert_eq!(anthropic.name, "anthropic");
    }

    #[test]
    fn returns_none_when_no_match() {
        let mut mgr = ProviderManager::new();

        let mut compat = Compatibility::default();
        compat.openai_chat = true;
        mgr.add(make_provider("openai", "https://example.com", compat));

        assert!(mgr.get_for_path("/v1/messages").is_none());
    }

    #[test]
    fn empty_manager_returns_none() {
        let mgr = ProviderManager::new();
        assert!(mgr.get_for_path("/v1/chat/completions").is_none());
    }
}
