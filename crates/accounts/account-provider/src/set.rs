//! The installed providers: the system's files, overlaid by the user's (user files win).

use crate::spec::{DomainMatch, DomainName, ProviderSpec};
use account_capability::Offer;
use account_core::ProviderId;
use account_match::{Match, Need, matches};

/// Every provider accountd knows, by id.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProviderSet {
    specs: Vec<ProviderSpec>,
}

impl ProviderSet {
    /// The system's providers with the user's laid over them: a user file with the id of a
    /// system file replaces it; the rest are added, in file order.
    #[must_use]
    pub fn layered(system: Vec<ProviderSpec>, user: Vec<ProviderSpec>) -> Self {
        let kept: Vec<ProviderSpec> = system
            .into_iter()
            .filter(|s| !user.iter().any(|u| u.id == s.id))
            .collect();
        Self {
            specs: kept.into_iter().chain(user).collect(),
        }
    }

    /// The provider with this id.
    #[must_use]
    pub fn get(&self, id: &ProviderId) -> Option<&ProviderSpec> {
        self.specs.iter().find(|spec| spec.id == *id)
    }

    /// Every provider, in order.
    #[must_use]
    pub fn specs(&self) -> &[ProviderSpec] {
        &self.specs
    }

    /// The providers that claim an address at `domain` whose MX hosts are `mx_hosts`, a listed
    /// domain before an MX hint, each in file order.
    #[must_use]
    pub fn claiming(
        &self,
        domain: &DomainName,
        mx_hosts: &[DomainName],
    ) -> Vec<(&ProviderSpec, DomainMatch)> {
        let claims = |wanted: DomainMatch| {
            self.specs
                .iter()
                .filter(move |spec| spec.matching.claims(domain, mx_hosts) == Some(wanted))
                .map(move |spec| (spec, wanted))
        };
        claims(DomainMatch::Domain)
            .chain(claims(DomainMatch::Mx))
            .collect()
    }

    /// Whether any provider declares a capability that meets `need`, so adding an account
    /// could help.
    #[must_use]
    pub fn offers(&self, need: &Need) -> bool {
        let declared = self.specs.iter().flat_map(|spec| &spec.capabilities);
        declared
            .map(|row| Offer::Present(row.capability.clone()))
            .any(|offer| matches(need, &offer) == Match::Fits)
    }
}
