//! Graph-enhanced RAG (Retrieval-Augmented Generation).
//!
//! This module combines baseline BM25 hits with dependency-graph traversal so
//! Kyro can answer both file-local and codebase-relationship questions.

use crate::repowiki::DependencyGraph;
use petgraph::algo::page_rank;
use petgraph::graph::{DiGraph, NodeIndex};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet, VecDeque};

/// Search modes inspired by GraphRAG query patterns.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum GraphSearchMode {
    /// Baseline BM25 plus direct graph expansion around matching files.
    Local,
    /// Community-style search that boosts hub files and wider traversals.
    Drift,
    /// Broad codebase search that surfaces structurally central files.
    Global,
}

/// A graph-enhanced search result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GraphSearchResult {
    pub file_path: String,
    pub content: String,
    pub bm25_score: f32,
    pub graph_score: f32,
    pub combined_score: f32,
    pub line_start: u32,
    pub line_end: u32,
    pub context: String,
    pub source: ResultSource,
    pub graph_distance: Option<u32>,
    pub neighbors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ResultSource {
    Direct,
    GraphNeighbor,
    Community,
}

#[derive(Debug, Clone)]
struct ScoredGraphNode {
    bm25_score: f32,
    graph_score: f32,
    combined_score: f32,
    distance: Option<u32>,
    source: ResultSource,
}

#[derive(Debug, Clone)]
struct GraphTopology {
    outgoing: HashMap<String, Vec<String>>,
    undirected: HashMap<String, Vec<String>>,
    page_rank: HashMap<String, f32>,
}

/// Expand BM25 search results with graph traversal and re-rank.
pub fn graph_enhanced_search(
    bm25_results: Vec<(String, f32, String, u32, u32, String)>,
    graph: &DependencyGraph,
    max_results: usize,
    mode: GraphSearchMode,
) -> Vec<GraphSearchResult> {
    if bm25_results.is_empty() || max_results == 0 {
        return Vec::new();
    }

    let topology = build_topology(graph);
    let direct_hits: HashMap<String, (f32, String, u32, u32, String)> = bm25_results
        .into_iter()
        .map(
            |(file_path, bm25_score, content, line_start, line_end, context)| {
                (
                    file_path,
                    (bm25_score, content, line_start, line_end, context),
                )
            },
        )
        .collect();

    let ranked_nodes = rank_graph_nodes(&direct_hits, graph, &topology, mode, max_results);

    ranked_nodes
        .into_iter()
        .map(|(file_path, scored)| {
            let (content, line_start, line_end, context) = direct_hits
                .get(&file_path)
                .map(|(_, content, line_start, line_end, context)| {
                    (content.clone(), *line_start, *line_end, context.clone())
                })
                .unwrap_or_else(|| {
                    let neighbor_context = match scored.distance {
                        Some(distance) if scored.source == ResultSource::Community => {
                            format!(
                                "Community summary candidate discovered {} hops away",
                                distance
                            )
                        }
                        Some(distance) => {
                            format!(
                                "Dependency graph neighbor discovered {} hops away",
                                distance
                            )
                        }
                        None => "Graph-derived context candidate".to_string(),
                    };
                    (String::new(), 0, 0, neighbor_context)
                });

            GraphSearchResult {
                neighbors: get_neighbors(graph, &file_path),
                file_path,
                content,
                bm25_score: scored.bm25_score,
                graph_score: scored.graph_score,
                combined_score: scored.combined_score,
                line_start,
                line_end,
                context,
                source: scored.source,
                graph_distance: scored.distance,
            }
        })
        .collect()
}

fn rank_graph_nodes(
    direct_hits: &HashMap<String, (f32, String, u32, u32, String)>,
    graph: &DependencyGraph,
    topology: &GraphTopology,
    mode: GraphSearchMode,
    max_results: usize,
) -> Vec<(String, ScoredGraphNode)> {
    let distances = shortest_distances(topology, direct_hits.keys().cloned().collect());
    let max_distance = match mode {
        GraphSearchMode::Local => 1,
        GraphSearchMode::Drift => 2,
        GraphSearchMode::Global => 3,
    };

    let max_page_rank = topology
        .page_rank
        .values()
        .copied()
        .fold(1.0f32, f32::max)
        .max(1.0);
    let max_degree = topology
        .undirected
        .values()
        .map(|neighbors| neighbors.len() as f32)
        .fold(1.0f32, f32::max)
        .max(1.0);

    let mut results = Vec::new();

    for node_path in graph.nodes.keys() {
        let direct = direct_hits.get(node_path);
        let distance = distances.get(node_path).copied();

        if direct.is_none() && distance.is_none() {
            continue;
        }

        if let Some(distance) = distance {
            if distance > max_distance {
                continue;
            }
        }

        let bm25_score = direct.map(|(score, ..)| *score).unwrap_or(0.0);
        let page_rank_score =
            topology.page_rank.get(node_path).copied().unwrap_or(0.0) / max_page_rank;
        let degree_score = topology
            .undirected
            .get(node_path)
            .map(|neighbors| neighbors.len() as f32 / max_degree)
            .unwrap_or(0.0);
        let proximity_score = distance
            .map(|value| 1.0 / (value as f32 + 1.0))
            .unwrap_or(0.0);

        let graph_score = match mode {
            GraphSearchMode::Local => {
                degree_score * 0.35 + page_rank_score * 0.2 + proximity_score * 0.45
            }
            GraphSearchMode::Drift => {
                degree_score * 0.25 + page_rank_score * 0.35 + proximity_score * 0.40
            }
            GraphSearchMode::Global => {
                degree_score * 0.20 + page_rank_score * 0.60 + proximity_score * 0.20
            }
        };

        let combined_score = if direct.is_some() {
            match mode {
                GraphSearchMode::Local => bm25_score * 0.7 + graph_score * 0.3,
                GraphSearchMode::Drift => bm25_score * 0.55 + graph_score * 0.45,
                GraphSearchMode::Global => bm25_score * 0.4 + graph_score * 0.6,
            }
        } else {
            match mode {
                GraphSearchMode::Local => graph_score * 0.55,
                GraphSearchMode::Drift => graph_score * 0.75,
                GraphSearchMode::Global => graph_score,
            }
        };

        let source = if direct.is_some() {
            ResultSource::Direct
        } else if mode == GraphSearchMode::Global {
            ResultSource::Community
        } else {
            ResultSource::GraphNeighbor
        };

        results.push((
            node_path.clone(),
            ScoredGraphNode {
                bm25_score,
                graph_score,
                combined_score,
                distance,
                source,
            },
        ));
    }

    results.sort_by(|left, right| {
        right
            .1
            .combined_score
            .partial_cmp(&left.1.combined_score)
            .unwrap_or(Ordering::Equal)
    });
    results.truncate(max_results);
    results
}

fn build_topology(graph: &DependencyGraph) -> GraphTopology {
    let mut directed_graph = DiGraph::<String, ()>::new();
    let mut node_map: HashMap<String, NodeIndex> = HashMap::new();
    let mut outgoing: HashMap<String, Vec<String>> = HashMap::new();
    let mut undirected: HashMap<String, Vec<String>> = HashMap::new();

    for path in graph.nodes.keys() {
        let index = directed_graph.add_node(path.clone());
        node_map.insert(path.clone(), index);
        outgoing.entry(path.clone()).or_default();
        undirected.entry(path.clone()).or_default();
    }

    for edge in &graph.edges {
        let Some(from) = node_map.get(&edge.from).copied() else {
            continue;
        };
        let Some(to) = node_map.get(&edge.to).copied() else {
            continue;
        };

        directed_graph.add_edge(from, to, ());
        outgoing
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        undirected
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        undirected
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
    }

    let mut page_rank_scores = HashMap::new();
    let scores = page_rank(&directed_graph, 0.85, 25);
    for index in directed_graph.node_indices() {
        if let Some(path) = directed_graph.node_weight(index) {
            page_rank_scores.insert(path.clone(), scores[index.index()] as f32);
        }
    }

    GraphTopology {
        outgoing,
        undirected,
        page_rank: page_rank_scores,
    }
}

fn shortest_distances(topology: &GraphTopology, seeds: Vec<String>) -> HashMap<String, u32> {
    let mut distances = HashMap::new();
    let mut queue = VecDeque::new();

    for seed in seeds {
        distances.insert(seed.clone(), 0);
        queue.push_back(seed);
    }

    while let Some(current) = queue.pop_front() {
        let distance = distances.get(&current).copied().unwrap_or(0);
        if let Some(neighbors) = topology.undirected.get(&current) {
            for neighbor in neighbors {
                if distances.contains_key(neighbor) {
                    continue;
                }
                distances.insert(neighbor.clone(), distance + 1);
                queue.push_back(neighbor.clone());
            }
        }
    }

    distances
}

/// Get immediate dependency neighbors for a given file path.
pub fn get_neighbors(graph: &DependencyGraph, file_path: &str) -> Vec<String> {
    let mut neighbors = HashSet::new();
    for edge in &graph.edges {
        if edge.from == file_path {
            neighbors.insert(edge.to.clone());
        } else if edge.to == file_path {
            neighbors.insert(edge.from.clone());
        }
    }
    let mut values: Vec<String> = neighbors.into_iter().collect();
    values.sort();
    values
}

#[cfg(test)]
mod tests {
    use super::{get_neighbors, graph_enhanced_search, GraphSearchMode, ResultSource};
    use crate::repowiki::{DepEdgeType, DependencyGraph, GraphEdge, GraphNode};
    use std::collections::HashMap;

    fn sample_graph() -> DependencyGraph {
        let mut nodes = HashMap::new();
        for path in ["src/app.rs", "src/parser.rs", "src/graph.rs", "src/ui.rs"] {
            nodes.insert(
                path.to_string(),
                GraphNode {
                    path: path.to_string(),
                    language: "rust".to_string(),
                    symbol_count: 4,
                    line_count: 20,
                    module_group: "src".to_string(),
                },
            );
        }

        DependencyGraph {
            nodes,
            edges: vec![
                GraphEdge {
                    from: "src/app.rs".to_string(),
                    to: "src/parser.rs".to_string(),
                    edge_type: DepEdgeType::Imports,
                    items: vec!["parse".to_string()],
                },
                GraphEdge {
                    from: "src/parser.rs".to_string(),
                    to: "src/graph.rs".to_string(),
                    edge_type: DepEdgeType::Imports,
                    items: vec!["build_graph".to_string()],
                },
                GraphEdge {
                    from: "src/ui.rs".to_string(),
                    to: "src/app.rs".to_string(),
                    edge_type: DepEdgeType::Imports,
                    items: vec!["run_app".to_string()],
                },
            ],
        }
    }

    #[test]
    fn local_mode_expands_direct_neighbors() {
        let graph = sample_graph();
        let results = graph_enhanced_search(
            vec![(
                "src/app.rs".to_string(),
                2.5,
                "fn run_app() {}".to_string(),
                1,
                1,
                "run_app".to_string(),
            )],
            &graph,
            3,
            GraphSearchMode::Local,
        );

        assert_eq!(results[0].file_path, "src/app.rs");
        assert_eq!(results[0].source, ResultSource::Direct);
        assert!(results.iter().any(|result| {
            result.file_path == "src/parser.rs" && result.source == ResultSource::GraphNeighbor
        }));
    }

    #[test]
    fn global_mode_surfaces_community_candidates() {
        let graph = sample_graph();
        let results = graph_enhanced_search(
            vec![(
                "src/app.rs".to_string(),
                1.0,
                "fn run_app() {}".to_string(),
                1,
                1,
                "run_app".to_string(),
            )],
            &graph,
            4,
            GraphSearchMode::Global,
        );

        assert!(results.iter().any(|result| {
            result.file_path == "src/graph.rs" && result.source == ResultSource::Community
        }));
    }

    #[test]
    fn neighbors_are_sorted_and_unique() {
        let graph = sample_graph();
        assert_eq!(
            get_neighbors(&graph, "src/app.rs"),
            vec!["src/parser.rs".to_string(), "src/ui.rs".to_string()]
        );
    }
}
