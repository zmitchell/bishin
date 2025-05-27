use std::{
    collections::HashMap,
    ops::Index,
    path::{Path, PathBuf},
};

use petgraph::{
    Direction::Outgoing,
    Graph,
    acyclic::Acyclic,
    graph::{DiGraph, NodeIndex},
    visit::IntoNeighborsDirected,
};
use walkdir::WalkDir;

/// The file extension of a bishin file.
pub const FILE_EXTENSION: &str = "b";

/// Errors collecting a module graph from the filesystem.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("failed to access test file or directory")]
    Walk(#[source] walkdir::Error),
    #[error("failed to read tests from directory '{0}'")]
    ReadRootDir(PathBuf, #[source] std::io::Error),
    #[error("failed to read directory entry")]
    ReadEntry(#[source] std::io::Error),
    #[error("no tests found")]
    Empty,
    #[error("{0}")]
    Other(String),
}

/// The modules that make up a test suite.
#[derive(Debug, Clone)]
struct ProtoModuleGraph {
    root: NodeIndex,
    graph: Acyclic<DiGraph<ProtoModule, ()>>,
}

impl ProtoModuleGraph {
    fn to_module_graph(&self) -> ModuleGraph {
        let mut paths = vec![];
        let mut stack = vec![(vec![self.root], self.root)];
        let mut map = HashMap::new();
        while let Some((path, node)) = stack.pop() {
            // Cache the mapping from node to module path
            map.insert(node, path.clone());

            // Only leaf nodes should have a file field populated.
            if self.graph[node].file.is_some() {
                debug_assert_eq!(self.graph.neighbors_directed(node, Outgoing).count(), 0);
                paths.push((path.clone(), self.graph[node].clone()));
                continue;
            }

            // For nodes that aren't leaves, collect their children.
            let children = self
                .graph
                .neighbors_directed(node, Outgoing)
                .collect::<Vec<_>>();
            for child in children.iter() {
                let new_path = {
                    let mut new_path = path.clone();
                    new_path.push(*child);
                    new_path
                };
                stack.push((new_path, *child));
            }
        }

        let new_graph = self.graph.filter_map(
            |idx, proto_module| {
                let path = map[&idx]
                    .iter()
                    .skip(1)
                    .map(|nid| self.graph[*nid].name.clone())
                    .collect::<Vec<_>>();
                let module = Module {
                    module_path: path,
                    file: proto_module.file.clone(),
                };
                Some(module)
            },
            |_, _| Some(()),
        );
        // We haven't changed any edges, so this should be safe.
        let acyclic = Acyclic::try_from_graph(new_graph).unwrap();

        ModuleGraph {
            _root: self.root,
            graph: acyclic,
        }
    }
}

/// A test module, which can either contain tests itself or other modules.
#[derive(Debug, Clone)]
struct ProtoModule {
    name: String,
    file: Option<PathBuf>,
}

#[derive(Debug, Clone)]
pub struct ModuleGraph {
    _root: NodeIndex,
    graph: Acyclic<DiGraph<Module, ()>>,
}

impl ModuleGraph {
    /// Iterates over the modules in the graph in topological order.
    ///
    /// Note that this will iterate over modules that themselves do not contain
    /// any tests. To only iterate over leaf modules, see `iter_leaf_modules`.
    pub fn iter_modules(&self) -> impl Iterator<Item = &Module> {
        let mut modules = self
            .graph
            .nodes_iter()
            .skip(1) // the root module
            .map(|nid| {
                // This bullshit is necessary because different graph types have
                // different Index implementations, some of which return owned
                // types, and others of which return references (we want references
                // in this case).
                let inner = self.graph.inner();
                <Graph<_, _, _, _> as Index<NodeIndex>>::index(inner, nid)
            })
            .collect::<Vec<_>>();
        modules.sort_by_key(|m| m.module_path());
        modules.into_iter()
    }

    /// Iterates over the modules in the graph in topological order.
    ///
    /// This only iterates over the modules that contain tests e.g. the modules
    /// that correspond to test files.
    pub fn iter_leaf_modules(&self) -> impl Iterator<Item = &Module> {
        let mut modules = self
            .graph
            .nodes_iter()
            .skip(1) // the root module
            .map(|nid| {
                // This bullshit is necessary because different graph types have
                // different Index implementations, some of which return owned
                // types, and others of which return references (we want references
                // in this case).
                let inner = self.graph.inner();
                <Graph<_, _, _, _> as Index<NodeIndex>>::index(inner, nid)
            })
            .filter(|module| module.file.is_some())
            .collect::<Vec<_>>();
        modules.sort_by_key(|m| m.module_path());
        modules.into_iter()
    }
}

#[derive(Debug, Clone)]
pub struct Module {
    module_path: Vec<String>,
    file: Option<PathBuf>,
}

impl Module {
    /// Returns the name of this module.
    pub fn name(&self) -> String {
        debug_assert!(!self.module_path.is_empty());
        self.module_path.last().cloned().unwrap()
    }

    /// Returns the path to the original file this module came from.
    pub fn file_path(&self) -> Option<PathBuf> {
        self.file.clone()
    }

    /// Returns the components of the module path.
    pub fn module_path_components(&self) -> Vec<String> {
        self.module_path.clone()
    }

    /// Returns the module path as a string.
    pub fn module_path(&self) -> String {
        self.module_path.join("::")
    }
}

/// Load a module graph rooted in a particular directory.
pub fn load_tests(root_path: impl AsRef<Path>) -> Result<ModuleGraph, Error> {
    let mut graph = DiGraph::new();
    let mut path_to_node = HashMap::new();
    let mut node_to_path = HashMap::new();
    let mut root = None;
    // First populate all nodes
    for entry in WalkDir::new(root_path) {
        let entry = entry.map_err(Error::Walk)?;
        let path = entry.path().to_path_buf();
        let file = if path.is_file() {
            if path.extension().is_some_and(|e| e == FILE_EXTENSION) {
                Some(path.clone())
            } else {
                None
            }
        } else {
            None
        };
        let mut module = ProtoModule {
            name: entry
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            file,
        };
        if root.is_none() {
            module.name = "root".to_string();
        }
        let idx = graph.add_node(module);
        if root.is_none() {
            root = Some(idx);
        }
        path_to_node.insert(path.clone(), idx);
        node_to_path.insert(idx, path);
    }
    // Then add edges
    for node_idx in graph.node_indices() {
        let path = node_to_path.get(&node_idx).unwrap();
        if let Some(parent) = path.parent() {
            if let Some(parent_node) = path_to_node.get(parent) {
                graph.add_edge(*parent_node, node_idx, ());
            }
        }
    }
    // Then remove any leaf nodes that are directories without children
    let graph = graph.filter_map(
        |node_idx, module| {
            // Pass test files straight through.
            if module.file.is_some() {
                return Some(module.clone());
            }
            // For directories we only want to keep them if they contain
            // test files.
            if graph
                .edges_directed(node_idx, petgraph::Direction::Outgoing)
                .count()
                == 0
            {
                None
            } else {
                Some(module.clone())
            }
        },
        |_edge_idx, _edge_weight| Some(()),
    );
    let graph = Acyclic::try_from_graph(graph).map_err(|_| {
        Error::Other("internal error: cycle detected constructing module graph".to_string())
    })?;
    if root.is_none() {
        return Err(Error::Empty);
    }
    let proto_graph = ProtoModuleGraph {
        root: root.unwrap(),
        graph,
    };
    Ok(proto_graph.to_module_graph())
}

#[cfg(test)]
mod tests {
    use expect_test::expect;
    use tempfile::TempDir;

    use super::*;

    fn print_whole_module_graph(graph: &ModuleGraph) -> String {
        let paths = graph
            .iter_modules()
            .map(|module| module.module_path())
            .inspect(|path| assert!(!path.is_empty()))
            .collect::<Vec<_>>();
        paths.join("\n")
    }

    fn print_leaf_modules(graph: &ModuleGraph) -> String {
        let paths = graph
            .iter_leaf_modules()
            .map(|module| module.module_path())
            .collect::<Vec<_>>();
        paths.join("\n")
    }

    #[test]
    fn loads_files_without_error() {
        let tempdir = TempDir::new().unwrap();
        let files = ["foo.b", "bar.b"];
        for f in files.iter() {
            std::fs::File::create(tempdir.path().join(f)).unwrap();
        }
        let modules = load_tests(tempdir.path()).unwrap();
        let printed_graph = print_whole_module_graph(&modules);
        eprintln!("{printed_graph}");
        let expected = expect![[r#"
            foo
            bar"#]];
        expected.assert_eq(&printed_graph);
    }

    #[test]
    fn loads_files_and_directories_without_error() {
        let tempdir = TempDir::new().unwrap();
        std::fs::create_dir(tempdir.path().join("subdir")).unwrap();
        let files = ["foo.b", "bar.b", "subdir/baz.b"];
        for f in files.iter() {
            std::fs::File::create(tempdir.path().join(f)).unwrap();
        }
        let modules = load_tests(tempdir.path()).unwrap();
        let printed_graph = print_whole_module_graph(&modules);
        eprintln!("{printed_graph}");
        let expected = expect![[r#"
            bar
            foo
            subdir
            subdir::baz"#]];
        expected.assert_eq(&printed_graph);
    }

    #[test]
    fn iterates_leaf_modules() {
        let tempdir = TempDir::new().unwrap();
        std::fs::create_dir(tempdir.path().join("subdir")).unwrap();
        let files = ["foo.b", "bar.b", "subdir/baz.b"];
        for f in files.iter() {
            std::fs::File::create(tempdir.path().join(f)).unwrap();
        }
        let modules = load_tests(tempdir.path()).unwrap();
        let printed_graph = print_leaf_modules(&modules);
        eprintln!("{printed_graph}");
        let expected = expect![[r#"
            bar
            foo
            subdir::baz"#]];
        expected.assert_eq(&printed_graph);
    }
}
