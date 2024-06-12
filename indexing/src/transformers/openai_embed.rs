use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures_util::{stream, StreamExt};

use crate::{
    ingestion_node::IngestionNode,
    ingestion_pipeline::IngestionStream,
    traits::{BatchableTransformer, Embed},
};

#[derive(Debug)]
// TODO: Would be nice if the embedding model encapsulates the token limit
pub struct EmbeddingModel(String);

#[derive(Debug)]
pub struct OpenAIEmbed {
    client: Arc<dyn Embed>,
    model: EmbeddingModel,
}

impl OpenAIEmbed {
    pub fn new(model: impl Into<EmbeddingModel>, client: Arc<dyn Embed>) -> Self {
        Self {
            client,
            model: model.into(),
        }
    }
}

#[async_trait]
impl BatchableTransformer for OpenAIEmbed {
    #[tracing::instrument(skip_all, name = "transformers.openai_embed")]
    async fn batch_transform(&self, nodes: Vec<IngestionNode>) -> IngestionStream {
        // TODO: We should drop chunks that go over the token limit of the EmbedModel
        let chunks_to_embed: Vec<String> = nodes.iter().map(|n| n.as_embeddable()).collect();

        stream::iter(
            self.client
                .embed(chunks_to_embed)
                .await
                .map(|embeddings| {
                    nodes
                        .into_iter()
                        .zip(embeddings)
                        .map(|(mut n, v)| {
                            n.vector = Some(v);
                            Ok(n)
                        })
                        .collect::<Vec<Result<IngestionNode>>>()
                })
                .unwrap_or_else(|e| vec![Err(e)]),
        )
        .boxed()
    }
}

#[allow(clippy::to_string_trait_impl)]
impl ToString for EmbeddingModel {
    fn to_string(&self) -> String {
        self.0.clone()
    }
}

impl From<&str> for EmbeddingModel {
    fn from(val: &str) -> Self {
        EmbeddingModel(val.to_string())
    }
}
