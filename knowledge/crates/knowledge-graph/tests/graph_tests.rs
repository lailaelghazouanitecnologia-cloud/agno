use knowledge_graph::*;

fn sample_graph() -> KnowledgeGraph {
    let mut g = KnowledgeGraph::new();

    // Create nodes
    g.add_node(
        Node::new("project", "Agno Project", NodeKind::Concept)
            .with_description("The root project node")
            .with_weight(1.0),
    )
    .unwrap();

    g.add_node(
        Node::new("kkr", "KKR Agent Framework", NodeKind::Module)
            .with_description("Agent loop, providers, tools")
            .with_tag("core"),
    )
    .unwrap();

    g.add_node(
        Node::new("knowledge", "Knowledge System", NodeKind::Module)
            .with_description("Graph-based project understanding")
            .with_tag("core"),
    )
    .unwrap();

    g.add_node(
        Node::new("roska", "Roska Code Analysis", NodeKind::Module)
            .with_description("Tree-sitter based code descriptors"),
    )
    .unwrap();

    g.add_node(
        Node::new("auth", "Authentication", NodeKind::Feature)
            .with_tag("security"),
    )
    .unwrap();

    // Create edges
    g.add_edge(Edge::new("project", "kkr", EdgeRelation::Parent))
        .unwrap();
    g.add_edge(Edge::new("project", "knowledge", EdgeRelation::Parent))
        .unwrap();
    g.add_edge(Edge::new("project", "roska", EdgeRelation::Parent))
        .unwrap();
    g.add_edge(Edge::new("knowledge", "roska", EdgeRelation::DependsOn))
        .unwrap();
    g.add_edge(Edge::new("kkr", "auth", EdgeRelation::Related))
        .unwrap();

    g
}

#[test]
fn add_and_get_nodes() {
    let g = sample_graph();
    assert_eq!(g.node_count(), 5);
    assert!(g.get_node("kkr").is_some());
    assert!(g.get_node("nonexistent").is_none());
}

#[test]
fn add_and_get_edges() {
    let g = sample_graph();
    assert_eq!(g.edge_count(), 5);

    let from_project = g.edges_from("project");
    assert_eq!(from_project.len(), 3); // kkr, knowledge, roska

    let to_roska = g.edges_to("roska");
    assert_eq!(to_roska.len(), 2); // project->roska, knowledge->roska
}

#[test]
fn navigation() {
    let g = sample_graph();

    // Children of project
    let children = g.children("project");
    assert_eq!(children.len(), 3);

    // Parents of kkr
    let parents = g.parents("kkr");
    assert_eq!(parents.len(), 1);
    assert_eq!(parents[0].id, "project");

    // Neighbors of knowledge
    let neighbors = g.neighbors("knowledge");
    assert_eq!(neighbors.len(), 1); // roska (via DependsOn)

    // Descendants of project
    let desc = g.descendants("project");
    assert_eq!(desc.len(), 3); // kkr, knowledge, roska
}

#[test]
fn ancestors() {
    let g = sample_graph();
    let ancestors = g.ancestors("kkr");
    assert_eq!(ancestors.len(), 1);
    assert_eq!(ancestors[0].id, "project");
}

#[test]
fn conversations() {
    let mut g = sample_graph();

    let mut conv = Conversation::new("kkr", "Tool architecture");
    conv.add_message(MessageRole::User, "How should tools be structured?");
    conv.add_message(MessageRole::Agent, "Use the KKR Tool trait.");
    conv.add_outcome("Tools implement the KKR Tool trait");
    g.add_conversation(conv).unwrap();

    let convs = g.conversations_for("kkr");
    assert_eq!(convs.len(), 1);
    assert_eq!(convs[0].topic, "Tool architecture");
    assert_eq!(convs[0].message_count(), 2);

    // Node should be updated
    assert_eq!(g.get_node("kkr").unwrap().conversation_count, 1);
}

#[test]
fn decisions() {
    let mut g = sample_graph();

    let mut decision = Decision::new("knowledge", "Storage format", "Need persistent storage");
    decision.add_alternative(
        "YAML files",
        vec!["human readable".into()],
        vec!["slower".into()],
        true,
    );
    decision.accept();
    g.add_decision(decision).unwrap();

    let decisions = g.decisions_for("knowledge");
    assert_eq!(decisions.len(), 1);
    assert_eq!(decisions[0].status, DecisionStatus::Accepted);
}

#[test]
fn specs() {
    let mut g = sample_graph();

    let mut spec = Spec::new("auth", "JWT Authentication")
        .with_description("Implement JWT-based auth");
    spec.add_criterion("Tokens expire after 1 hour");
    spec.add_criterion("Refresh tokens supported");
    g.add_spec(spec).unwrap();

    let specs = g.specs_for("auth");
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].acceptance_criteria.len(), 2);
}

#[test]
fn find_nodes() {
    let g = sample_graph();

    let modules = g.find_nodes(&NodeQuery::new().kind(NodeKind::Module));
    assert_eq!(modules.len(), 3);

    let core = g.find_nodes(&NodeQuery::new().tag("core"));
    assert_eq!(core.len(), 2);

    let search = g.find_nodes(&NodeQuery::new().text("agent"));
    assert_eq!(search.len(), 1);
    assert_eq!(search[0].id, "kkr");
}

#[test]
fn remove_node() {
    let mut g = sample_graph();
    assert_eq!(g.node_count(), 5);

    g.remove_node("roska");
    assert_eq!(g.node_count(), 4);
    assert!(g.get_node("roska").is_none());

    // Edges involving roska should be gone
    let from_project = g.edges_from("project");
    assert_eq!(from_project.len(), 2); // only kkr, knowledge now
}

#[test]
fn context_rendering() {
    let mut g = sample_graph();

    let mut conv = Conversation::new("kkr", "Architecture overview");
    conv.add_outcome("KKR is the agent core");
    let conv = conv.with_summary("Discussed KKR's role as the central agent framework");
    g.add_conversation(conv).unwrap();

    let ctx = g.render_context("kkr", 1);
    assert!(ctx.contains("KKR Agent Framework"));
    assert!(ctx.contains("Context (parents)"));
    assert!(ctx.contains("Architecture overview"));
}

#[test]
fn map_rendering() {
    let g = sample_graph();
    let map = g.render_map();
    assert!(map.contains("5 nodes"));
    assert!(map.contains("5 edges"));
}

#[test]
fn related_by() {
    let g = sample_graph();
    let deps = g.related_by("knowledge", EdgeRelation::DependsOn);
    assert_eq!(deps.len(), 1);
    assert_eq!(deps[0].id, "roska");
}
