use crate::{BatchableTransformer, ChunkerTransformer, Loader, NodeCache, Storage, Transformer};
use anyhow::Result;
use futures_util::{StreamExt, TryFutureExt, TryStreamExt};

use std::sync::Arc;

use super::{IngestionNode, IngestionStream};

pub struct IngestionPipeline {
    stream: IngestionStream,
    storage: Option<Box<dyn Storage>>,
    concurrency: usize,
}

// A lazy pipeline for ingesting files, adding metadata, chunking, transforming, embedding and then storing them.
impl IngestionPipeline {
    pub fn from_loader(loader: impl Loader + 'static) -> Self {
        let stream = loader.into_stream();
        Self {
            stream: stream.boxed(),
            storage: None,
            concurrency: 10,
        }
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn filter_cached(mut self, cache: impl NodeCache + 'static) -> Self {
        let cache = Arc::new(cache);
        self.stream = self
            .stream
            .try_filter(move |node| {
                let cache = Arc::clone(&cache);
                // FIXME: Maybe Cow or arc instead? Lots of nodes
                // Or we could get the key before the spawn
                let node = node.clone();
                let current_span = tracing::Span::current();
                tokio::spawn(current_span.in_scope(|| async move {
                    if !cache.get(&node).await {
                        cache.set(&node).await;

                        tracing::debug!("Node not in cache, passing through");

                        true
                    } else {
                        tracing::debug!("Node in cache, skipping");
                        false
                    }
                }))
                .unwrap_or_else(|e| {
                    tracing::error!("Error filtering cached node: {:?}", e);
                    true
                })
            })
            .boxed();
        self
    }

    pub fn then(mut self, transformer: impl Transformer + 'static) -> Self {
        let transformer = Arc::new(transformer);
        self.stream = self
            .stream
            .map_ok(move |node| {
                let transformer = Arc::clone(&transformer);
                let current_span = tracing::Span::current();
                tokio::spawn(
                    current_span.in_scope(|| async move { transformer.transform_node(node).await }),
                )
                .map_err(anyhow::Error::from)
            })
            .try_buffer_unordered(self.concurrency)
            // Flatten the double result
            .map(|x| x.and_then(|x| x))
            .boxed();

        self
    }

    pub fn then_in_batch(
        mut self,
        batch_size: usize,
        transformer: impl BatchableTransformer + 'static,
    ) -> Self {
        let transformer = Arc::new(transformer);
        self.stream = self
            .stream
            .try_chunks(batch_size)
            .map_ok(move |chunks| {
                let transformer = Arc::clone(&transformer);
                let current_span = tracing::Span::current();
                tokio::spawn(
                    current_span
                        .in_scope(|| async move { transformer.batch_transform(chunks).await }),
                )
                .map_err(anyhow::Error::from)
            })
            // We need to coerce both the stream error and tokio error to anyhow manually
            .err_into::<anyhow::Error>()
            .try_buffer_unordered(self.concurrency)
            .try_flatten()
            .boxed();
        self
    }

    // Takes a single node, splits it into multiple, then flattens the stream
    pub fn then_chunk(mut self, chunker: impl ChunkerTransformer + 'static) -> Self {
        let chunker = Arc::new(chunker);
        self.stream = self
            .stream
            .map_ok(move |node| {
                let chunker = Arc::clone(&chunker);
                let current_span = tracing::Span::current();
                tokio::spawn(
                    current_span.in_scope(|| async move { chunker.transform_node(node).await }),
                )
                .map_err(anyhow::Error::from)
            })
            // We need to coerce both the stream error and tokio error to anyhow manually
            .err_into::<anyhow::Error>()
            .try_buffer_unordered(self.concurrency)
            .try_flatten()
            .boxed();

        self
    }

    pub fn store_with(mut self, storage: impl Storage + 'static) -> Self {
        self.storage = Some(Box::new(storage));
        self
    }

    #[tracing::instrument(skip_all, fields(total_nodes), name = "ingestion_pipeline.run")]
    pub async fn run(mut self) -> Result<()> {
        let Some(ref storage) = self.storage else {
            anyhow::bail!("No storage configured for ingestion pipeline")
        };

        storage.setup().await?;

        let mut total_nodes = 0;
        if let Some(batch_size) = storage.batch_size() {
            // Chunk both Ok and Err results, early return on any error
            let mut stream = self.stream.chunks(batch_size).boxed();
            while let Some(nodes) = stream.next().await {
                let nodes = nodes.into_iter().collect::<Result<Vec<IngestionNode>>>()?;
                total_nodes += nodes.len();
                storage.batch_store(nodes).await?;
            }
        } else {
            while let Some(node) = self.stream.next().await {
                total_nodes += 1;
                storage.store(node?).await?;
            }
        }

        tracing::Span::current().record("total_nodes", total_nodes);

        Ok(())
    }
}