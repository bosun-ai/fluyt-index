<details>
  <summary>Table of Contents</summary>
<!--toc:start-->

- [About The Project](#about-the-project)
- [Example](#example)
- [Features](#features)
- [Vision](#vision)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
- [Usage and concepts](#usage-and-concepts)
- [Roadmap](#roadmap)
- [Contributing](#contributing)
- [License](#license)
    <!--toc:end-->
  </details>

<a name="readme-top"></a>

<!-- PROJECT SHIELDS -->
<!--
*** I'm using markdown "reference style" links for readability.
*** Reference links are enclosed in brackets [ ] instead of parentheses ( ).
*** See the bottom of this document for the declaration of the reference variables
*** for contributors-url, forks-url, etc. This is an optional, concise syntax you may use.
*** https://www.markdownguide.org/basic-syntax/#reference-style-links
-->
![CI](https://img.shields.io/github/actions/workflow/status/bosun-ai/swiftide/test.yml?style=flat-square)
![Coverage Status](https://img.shields.io/coverallsCoverage/github/bosun-ai/swiftide?style=flat-square)
[![Crate Badge]][Crate]
[![Docs Badge]][API Docs]
[![Contributors][contributors-shield]][contributors-url]
[![Stargazers][stars-shield]][stars-url]
[![MIT License][license-shield]][license-url]
[![LinkedIn][linkedin-shield]][linkedin-url]

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="https://github.com/bosun-ai/swiftide">
    <img src="https://github.com/bosun-ai/swiftide/blob/master/images/logo.png" alt="Logo" width="250" height="250">
  </a>

<h3 align="center">Swiftide</h3>

  <p align="center">
Blazing fast data pipelines for Retrieval Augmented Generation written in Rust 
    <br />
    <a href="https://swiftide.rs"><strong>Explore the docs »</strong></a>
    <br />
    <br />
    <!-- <a href="https://github.com/bosun-ai/swiftide">View Demo</a> -->
    <a href="https://docs.rs/swiftide/latest/swiftide/">API Docs</a>
    ·
    <a href="https://github.com/bosun-ai/swiftide/issues/new?labels=bug&template=bug_report.md">Report Bug</a>
    ·
    <a href="https://github.com/bosun-ai/swiftide/issues/new?labels=enhancement&template=feature_request.md">Request Feature</a>
  </p>
</div>

<!-- ABOUT THE PROJECT -->

## About The Project

<!-- [![Product Name Screen Shot][product-screenshot]](https://example.com) -->

**Swiftide** is a straightforward, easy-to-use, easy-to-extend asynchronous data ingestion and processing library. It is designed to be used in a RAG (Retrieval Augmented Generation) system. It is built to be fast and efficient, with a focus on parallel processing and asynchronous operations.

<div align="center">
  <a href="https://github.com/bosun-ai/swiftide">
    <img src="https://github.com/bosun-ai/swiftide/blob/master/images/rag-dark.svg" alt="RAG" width="100%" >
  </a>
</div>

While working with other Python-based tooling, frustrations arose around performance, stability, and ease of use. Thus, Swiftide was born. Ingestion performance went from multiple tens of minutes to a few seconds.

Part of the [bosun.ai](https://bosun.ai) project. An upcoming platform for autonomous code improvement.

We <3 feedback: project ideas, suggestions, and complaints are very welcome. Feel free to open an issue.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Example

```rust
IngestionPipeline::from_loader(FileLoader::new(".").with_extensions(&["rs"]))
        .filter_cached(Redis::try_from_url(
            redis_url,
            "swiftide-examples",
        )?)
        .then(MetadataQACode::new(openai_client.clone()))
        .then_chunk(ChunkCode::try_for_language_and_chunk_size(
            "rust",
            10..2048,
        )?)
        .then_in_batch(10, Embed::new(openai_client.clone()))
        .then_store_with(
            Qdrant::try_from_url(qdrant_url)?
                .batch_size(50)
                .vector_size(1536)
                .collection_name("swiftide-examples".to_string())
                .build()?,
        )
        .run()
        .await?;
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Features

- Extremely fast streaming ingestion pipeline with async, parallel processing
- Integrations with OpenAI, Redis, Qdrant, FastEmbed, and Treesitter
- A variety of loaders, transformers, and embedders and other common, generic tools
- Bring your own transformers by extending straightforward traits
- Store into multiple backends
- `tracing` supported for logging and tracing, see /examples and the `tracing` crate for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Vision

Our goal is to create an extremely fast, extendable platform for data ingestion and querying to further the development of automated LLM applications, with an easy-to-use and easy-to-extend api.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->

## Getting Started

### Prerequisites

Make sure you have the rust toolchain installed. [rustup](https://rustup.rs) Is the recommended approach.

To use OpenAI, an API key is required. Note that by default `async_openai` uses the `OPENAI_API_KEY` environment variables.

Other integrations will need to be installed accordingly.

### Installation

1. Set up a new Rust project
2. Add swiftide
   ```sh
   cargo add swiftide
   ```
3. Enable the features of integrations you would like to have or use 'all' in your `Cargo.toml`
4. Write a pipeline (see our examples and documentation)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE EXAMPLES -->

## Usage and concepts

Before building your stream, you need to enable and configure any integrations required. See /examples.

A stream starts with a Loader that emits IngestionNodes. For instance, with the Fileloader each file is a Node.

You can then slice and dice, augment, and filter nodes. Each different kind of step in the pipeline requires different traits. This enables extension.

IngestionNodes have a path, chunk and metadata. Currently metadata is copied over when chunking and _always_ embedded when using the OpenAIEmbed transformer.

- **from_loader** `(impl Loader)` starting point of the stream, creates and emits IngestionNodes
- **filter_cached** `(impl NodeCache)` filters cached nodes
- **then** `(impl Transformer)` transforms the node and puts it on the stream
- **then_in_batch** `(impl BatchTransformer)` transforms multiple nodes and puts them on the stream
- **then_chunk** `(impl ChunkerTransformer)` transforms a single node and emits multiple nodes
- **then_store_with** `(impl Storage)` stores the nodes in a storage backend, this can be chained

Additionally, several generic transformers are implemented. They take implementers of `SimplePrompt` and `EmbedModel` to do their things.

> [!NOTE]
> No integrations are enabled by default as some are code heavy. Either cherry-pick the integrations you need or use the "all" feature flag.

> [!WARNING]
> Due to the performance, chunking before adding metadata gives rate limit errors on OpenAI very fast, especially with faster models like 3.5-turbo. Be aware.

_For more examples, please refer to /examples and the [Documentation](https://docs.rs/swiftide/latest/swiftide/)_

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ROADMAP -->

## Roadmap

See the [open issues](https://github.com/bosun-ai/swiftide/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTRIBUTING -->

## Contributing

Swiftide is in a very early stage and we are aware that we do lack features for the wider community. Contributions are very welcome. :tada:

If you have a great idea, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

If you just want to contribute (bless you!), see [our issues](https://github.com/bosun-ai/swiftide/issues).

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'feat: Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->

## License

Distributed under the MIT License. See `LICENSE` for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->

[contributors-shield]: https://img.shields.io/github/contributors/bosun-ai/swiftide.svg?style=flat-square
[contributors-url]: https://github.com/bosun-ai/swiftide/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/bosun-ai/swiftide.svg?style=flat-square
[forks-url]: https://github.com/bosun-ai/swiftide/network/members
[stars-shield]: https://img.shields.io/github/stars/bosun-ai/swiftide.svg?style=flat-square
[stars-url]: https://github.com/bosun-ai/swiftide/stargazers
[issues-shield]: https://img.shields.io/github/issues/bosun-ai/swiftide.svg?style=flat-square
[issues-url]: https://github.com/bosun-ai/swiftide/issues
[license-shield]: https://img.shields.io/github/license/bosun-ai/swiftide.svg?style=flat-square
[license-url]: https://github.com/bosun-ai/swiftide/blob/master/LICENSE.txt
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=flat-square&logo=linkedin&colorB=555
[linkedin-url]: https://www.linkedin.com/company/bosun-ai
[Crate Badge]: https://img.shields.io/crates/v/swiftide?logo=rust&style=flat-square&logoColor=E05D44&color=E05D44
[Crate]: https://crates.io/crates/swiftide
[Docs Badge]: https://img.shields.io/docsrs/swiftide?logo=rust&style=flat-square&logoColor=E05D44
[API Docs]: https://docs.rs/swiftide
