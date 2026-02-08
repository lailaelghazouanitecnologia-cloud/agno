use entity_knowledge::{
    ChunkingStrategy, ContentType, Document, FixedSizeChunking, MarkdownChunking,
    ReaderRegistry, RecursiveChunking, ScoreBasedReranker, Reranker, SentenceChunking,
};

// -- Document tests --

#[test]
fn test_document_creation() {
    let doc = Document::new("Hello, world!")
        .with_name("test.txt")
        .with_metadata("source", serde_json::json!("test"));

    assert_eq!(doc.content, "Hello, world!");
    assert_eq!(doc.name, Some("test.txt".to_string()));
    assert!(doc.metadata.contains_key("source"));
}

#[test]
fn test_content_hash() {
    let doc1 = Document::new("Hello");
    let doc2 = Document::new("Hello");
    let doc3 = Document::new("World");

    assert_eq!(doc1.content_hash(), doc2.content_hash());
    assert_ne!(doc1.content_hash(), doc3.content_hash());
}

#[test]
fn test_content_type_from_extension() {
    assert!(matches!(
        ContentType::from_extension("pdf"),
        ContentType::Pdf
    ));
    assert!(matches!(
        ContentType::from_extension("MD"),
        ContentType::Markdown
    ));
}

// -- Chunking tests --

#[test]
fn test_fixed_size_chunking() {
    let chunker = FixedSizeChunking::new(100, 20);
    let doc = Document::new("a".repeat(250));
    let chunks = chunker.chunk(doc).unwrap();

    assert!(chunks.len() >= 2);
    for chunk in &chunks {
        assert!(chunk.content.len() <= 100);
    }
}

#[test]
fn test_recursive_chunking() {
    let chunker = RecursiveChunking::new(50, 10);
    let doc = Document::new("This is sentence one. This is sentence two. This is sentence three.");
    let chunks = chunker.chunk(doc).unwrap();

    assert!(!chunks.is_empty());
}

#[test]
fn test_sentence_chunking() {
    let chunker = SentenceChunking::new(2, 0);
    let doc = Document::new("First sentence. Second sentence. Third sentence. Fourth sentence.");
    let chunks = chunker.chunk(doc).unwrap();

    assert_eq!(chunks.len(), 2);
}

#[test]
fn test_markdown_chunking() {
    let chunker = MarkdownChunking::new(100, 0);
    let doc = Document::new("# Header 1\nContent 1\n\n## Header 2\nContent 2");
    let chunks = chunker.chunk(doc).unwrap();

    assert!(!chunks.is_empty());
}

// -- Reranker tests --

#[tokio::test]
async fn test_score_based_reranker() {
    let reranker = ScoreBasedReranker::new().with_threshold(0.5);

    let documents = vec![
        Document::new("Doc 1").with_score(0.9),
        Document::new("Doc 2").with_score(0.3),
        Document::new("Doc 3").with_score(0.7),
    ];

    let result = reranker.rerank("query", documents, 10).await.unwrap();

    assert_eq!(result.len(), 2);
    assert_eq!(result[0].score, Some(0.9));
    assert_eq!(result[1].score, Some(0.7));
}

// -- Reader tests --

#[test]
fn test_reader_registry() {
    let registry = ReaderRegistry::new();
    assert!(registry.get("text").is_some());
    assert!(registry.get("json").is_some());
    assert!(registry.get("csv").is_some());
}
