//! `Index` as the service's search seam (`memory_store::SearchIndex`): each method is the
//! inherent one, with its error as the seam's.

use crate::doc::{Allow, Doc, DocId, Ranked, TopK};
use crate::embed::Embedder;
use crate::exact::VectorIndex;
use crate::index::{Index, SearchQuery};
use memory_core::IndexView;
use memory_core::Integrity;
use memory_store::{EmbedderCard, RrfK, SearchIndex, SearchIndexError, Vector};

impl<V: VectorIndex> SearchIndex for Index<V> {
    fn state(&self) -> IndexView {
        Index::<V>::state(self)
    }

    fn rrf_k(&self) -> RrfK {
        Index::<V>::rrf_k(self)
    }

    fn card(&self) -> &EmbedderCard {
        self.parts().1.card()
    }

    fn allow_kinds(
        &self,
        kinds: &[&str],
        trust: Option<Integrity>,
    ) -> Result<Allow, SearchIndexError> {
        Index::<V>::allow_kinds(self, kinds, trust).map_err(SearchIndexError::from)
    }

    fn lexical(&self, q: &SearchQuery) -> Result<Vec<Ranked>, SearchIndexError> {
        Index::<V>::lexical(self, q).map_err(SearchIndexError::from)
    }

    fn nearest(&self, v: &Vector, k: TopK, allow: &Allow) -> Result<Vec<Ranked>, SearchIndexError> {
        self.parts()
            .1
            .nearest(v, k, allow)
            .map_err(SearchIndexError::from)
    }

    async fn rebuild<E: Embedder>(
        &mut self,
        docs: Vec<Doc>,
        e: &E,
    ) -> Result<(), SearchIndexError> {
        Index::<V>::rebuild(self, docs.into_iter(), e)
            .await
            .map_err(SearchIndexError::from)
    }

    async fn sync<E: Embedder>(&mut self, docs: Vec<Doc>, e: &E) -> Result<(), SearchIndexError> {
        Index::<V>::sync(self, docs.into_iter(), e)
            .await
            .map_err(SearchIndexError::from)
    }

    async fn upsert<E: Embedder>(&mut self, docs: &[Doc], e: &E) -> Result<(), SearchIndexError> {
        Index::<V>::upsert(self, docs, e)
            .await
            .map_err(SearchIndexError::from)
    }

    fn remove(&mut self, ids: &[DocId]) -> Result<u32, SearchIndexError> {
        Index::<V>::remove(self, ids).map_err(SearchIndexError::from)
    }
}
