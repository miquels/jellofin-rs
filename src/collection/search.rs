use std::collections::HashMap;
use std::sync::Arc;
use tantivy::collector::TopDocs;
use tantivy::query::{BooleanQuery, FuzzyTermQuery, Occur, Query, QueryParser, TermQuery};
use tantivy::schema::*;
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, TantivyDocument};
use tokio::sync::RwLock;
use tracing::debug;

use super::collection::Collection;
use super::item::{Item, ItemType};

pub struct SearchIndex {
    index: Index,
    reader: IndexReader,
    writer: Arc<RwLock<IndexWriter>>,
    id_field: Field,
    collection_id_field: Field,
    name_field: Field,
    overview_field: Field,
    genres_field: Field,
    item_type_field: Field,
}

impl SearchIndex {
    pub fn new() -> Result<Self, SearchError> {
        let mut schema_builder = Schema::builder();

        let id_field = schema_builder.add_text_field("id", STRING | STORED);
        let collection_id_field = schema_builder.add_text_field("collection_id", STRING | STORED);
        let name_field = schema_builder.add_text_field("name", TEXT | STORED);
        let overview_field = schema_builder.add_text_field("overview", TEXT);
        let genres_field = schema_builder.add_text_field("genres", TEXT);
        let item_type_field = schema_builder.add_text_field("item_type", STRING | STORED);

        let schema = schema_builder.build();

        let index = Index::create_in_ram(schema.clone());

        let writer = index.writer(50_000_000)?;
        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        Ok(Self {
            index,
            reader,
            writer: Arc::new(RwLock::new(writer)),
            id_field,
            collection_id_field,
            name_field,
            overview_field,
            genres_field,
            item_type_field,
        })
    }

    pub async fn rebuild(
        &self,
        collections: &HashMap<String, Collection>,
    ) -> Result<(), SearchError> {
        debug!("Rebuilding search index");

        let mut writer = self.writer.write().await;
        writer.delete_all_documents()?;

        for collection in collections.values() {
            for movie in collection.movies.values() {
                let mut doc = TantivyDocument::default();
                doc.add_text(self.id_field, &movie.id);
                doc.add_text(self.collection_id_field, &movie.collection_id);
                doc.add_text(self.name_field, movie.name());

                if let Some(overview) = movie.overview() {
                    doc.add_text(self.overview_field, overview);
                }

                for genre in movie.genres() {
                    doc.add_text(self.genres_field, genre);
                }

                doc.add_text(self.item_type_field, ItemType::Movie.as_str());

                writer.add_document(doc)?;
            }

            for show in collection.shows.values() {
                let mut doc = TantivyDocument::default();
                doc.add_text(self.id_field, &show.id);
                doc.add_text(self.collection_id_field, &show.collection_id);
                doc.add_text(self.name_field, show.name());

                if let Some(overview) = show.overview() {
                    doc.add_text(self.overview_field, overview);
                }

                for genre in show.genres() {
                    doc.add_text(self.genres_field, genre);
                }

                doc.add_text(self.item_type_field, ItemType::Series.as_str());

                writer.add_document(doc)?;

                for season in show.seasons.values() {
                    for episode in season.episodes.values() {
                        let mut doc = TantivyDocument::default();
                        doc.add_text(self.id_field, &episode.id);
                        doc.add_text(self.collection_id_field, &episode.collection_id);
                        doc.add_text(self.name_field, episode.name());

                        if let Some(overview) = episode.overview() {
                            doc.add_text(self.overview_field, overview);
                        }

                        doc.add_text(self.item_type_field, ItemType::Episode.as_str());

                        writer.add_document(doc)?;
                    }
                }
            }
        }

        writer.commit()?;
        self.reader.reload()?;

        debug!("Search index rebuilt successfully");
        Ok(())
    }

    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![self.name_field, self.overview_field, self.genres_field],
        );

        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            if let (Some(id), Some(collection_id), Some(item_type)) = (
                retrieved_doc
                    .get_first(self.id_field)
                    .and_then(|v| v.as_str()),
                retrieved_doc
                    .get_first(self.collection_id_field)
                    .and_then(|v| v.as_str()),
                retrieved_doc
                    .get_first(self.item_type_field)
                    .and_then(|v| v.as_str()),
            ) {
                let name = retrieved_doc
                    .get_first(self.name_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                results.push(SearchResult {
                    id: id.to_string(),
                    collection_id: collection_id.to_string(),
                    item_type: item_type.to_string(),
                    name,
                });
            }
        }

        Ok(results)
    }

    pub fn find_similar(
        &self,
        item_id: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, SearchError> {
        let searcher = self.reader.searcher();

        let id_query = TermQuery::new(
            Term::from_field_text(self.id_field, item_id),
            IndexRecordOption::Basic,
        );

        let top_docs = searcher.search(&id_query, &TopDocs::with_limit(1))?;

        if top_docs.is_empty() {
            return Ok(Vec::new());
        }

        let (_score, doc_address) = top_docs[0];
        let source_doc: TantivyDocument = searcher.doc(doc_address)?;

        let genres: Vec<String> = source_doc
            .get_all(self.genres_field)
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect();

        let item_type = source_doc
            .get_first(self.item_type_field)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let mut subqueries: Vec<(Occur, Box<dyn Query>)> = Vec::new();

        for genre in &genres {
            let term = Term::from_field_text(self.genres_field, genre);
            let fuzzy_query = FuzzyTermQuery::new(term, 1, true);
            subqueries.push((Occur::Should, Box::new(fuzzy_query)));
        }

        let type_query = TermQuery::new(
            Term::from_field_text(self.item_type_field, item_type),
            IndexRecordOption::Basic,
        );
        subqueries.push((Occur::Must, Box::new(type_query)));

        let exclude_query = TermQuery::new(
            Term::from_field_text(self.id_field, item_id),
            IndexRecordOption::Basic,
        );
        subqueries.push((Occur::MustNot, Box::new(exclude_query)));

        let similar_query = BooleanQuery::new(subqueries);

        let top_docs = searcher.search(&similar_query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc: TantivyDocument = searcher.doc(doc_address)?;

            if let (Some(id), Some(collection_id), Some(item_type)) = (
                retrieved_doc
                    .get_first(self.id_field)
                    .and_then(|v| v.as_str()),
                retrieved_doc
                    .get_first(self.collection_id_field)
                    .and_then(|v| v.as_str()),
                retrieved_doc
                    .get_first(self.item_type_field)
                    .and_then(|v| v.as_str()),
            ) {
                let name = retrieved_doc
                    .get_first(self.name_field)
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                results.push(SearchResult {
                    id: id.to_string(),
                    collection_id: collection_id.to_string(),
                    item_type: item_type.to_string(),
                    name,
                });
            }
        }

        Ok(results)
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub collection_id: String,
    pub item_type: String,
    pub name: String,
}

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("Tantivy error: {0}")]
    Tantivy(#[from] tantivy::TantivyError),
    #[error("Query parse error: {0}")]
    QueryParse(#[from] tantivy::query::QueryParserError),
}
