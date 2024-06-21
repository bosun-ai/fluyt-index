//! # [Swiftide] Ingesting the Swiftide itself example
//!
//! This example demonstrates how to ingest the Swiftide codebase itself using FastEmbed.
//!
//! The pipeline will:
//! - Load all `.rs` files from the current directory
//! - Embed the chunks in batches of 10 using FastEmbed
//! - Store the nodes in Qdrant
//!
//! [Swiftide]: https://github.com/bosun-ai/swiftide
//! [examples]: https://github.com/bosun-ai/swiftide/blob/master/examples

use swiftide::{
    ingestion,
    integrations::{fastembed::FastEmbed, qdrant::Qdrant},
    loaders::FileLoader,
    transformers::Embed,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let qdrant_url = std::env::var("QDRANT_URL")
        .as_deref()
        .unwrap_or("http://localhost:6334")
        .to_owned();

    ingestion::IngestionPipeline::from_loader(FileLoader::new(".").with_extensions(&["rs"]))
        .then_in_batch(10, Embed::new(FastEmbed::builder().batch_size(10).build()?))
        .then_store_with(
            Qdrant::try_from_url(qdrant_url)?
                .batch_size(50)
                .vector_size(384)
                .collection_name("swiftide-examples-fastembed".to_string())
                .build()?,
        )
        .run()
        .await?;
    Ok(())
}
