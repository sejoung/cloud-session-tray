use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};
use syn::{Attribute, Item, ItemMod, Meta, UseTree, punctuated::Punctuated, visit::Visit};

type Module = Vec<String>;
type Graph = BTreeMap<Module, BTreeSet<Module>>;

#[test]
fn application_modules_follow_dependency_rules() {
    let mut modules = BTreeMap::new();
    let entry = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/main.rs");
    read_modules(Vec::new(), &entry, &mut modules);
    let (graph, mut violations) = dependency_graph(&modules);
    if let Some(cycle) = find_cycle(&graph) {
        let cycle = cycle.iter().map(|module| name(module)).collect::<Vec<_>>();
        violations.push(format!("Circular dependency: {}", cycle.join(" -> ")));
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

fn name(module: &[String]) -> String {
    if module.is_empty() {
        "main".into()
    } else {
        module.join("::")
    }
}

fn read_modules(module: Module, file: &Path, modules: &mut BTreeMap<Module, Vec<Item>>) {
    let contents = fs::read_to_string(file)
        .unwrap_or_else(|error| panic!("Cannot read source module {}: {error}", file.display()));
    let items = syn::parse_file(&contents)
        .unwrap_or_else(|error| panic!("Cannot parse source module {}: {error}", file.display()))
        .items;
    let directory = match file.file_name().and_then(|name| name.to_str()) {
        Some("main.rs" | "mod.rs") => file.parent().unwrap().to_path_buf(),
        _ => file.with_extension(""),
    };
    index_modules(module, &directory, items, modules);
}

fn index_modules(
    module: Module,
    directory: &Path,
    items: Vec<Item>,
    modules: &mut BTreeMap<Module, Vec<Item>>,
) {
    let mut source_items = Vec::new();
    for item in items {
        if let Item::Mod(item) = item {
            if !test_only(&item.attrs) {
                read_child(&module, directory, item, modules);
            }
        } else {
            source_items.push(item);
        }
    }
    modules.insert(module, source_items);
}

fn read_child(
    parent: &[String],
    directory: &Path,
    item: ItemMod,
    modules: &mut BTreeMap<Module, Vec<Item>>,
) {
    let child = item.ident.to_string();
    let mut module = parent.to_vec();
    module.push(child.clone());

    if let Some((_, items)) = item.content {
        index_modules(module, &directory.join(child), items, modules);
        return;
    }
    let direct = directory.join(format!("{child}.rs"));
    let file = if direct.is_file() {
        direct
    } else {
        directory.join(child).join("mod.rs")
    };
    read_modules(module, &file, modules);
}

fn test_only(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        attribute.path().is_ident("cfg") && attribute.parse_args::<Meta>().is_ok_and(requires_test)
    })
}

fn requires_test(condition: Meta) -> bool {
    match condition {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") => list
            .parse_args_with(Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            .is_ok_and(|items| items.into_iter().any(requires_test)),
        _ => false,
    }
}

#[derive(Default)]
struct References {
    paths: BTreeSet<Vec<String>>,
}

impl References {
    fn use_tree(&mut self, prefix: &mut Vec<String>, tree: &UseTree) {
        match tree {
            UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.use_tree(prefix, &path.tree);
                prefix.pop();
            }
            UseTree::Name(name) if name.ident != "self" => self.use_name(prefix, &name.ident),
            UseTree::Rename(rename) if rename.ident != "self" => {
                self.use_name(prefix, &rename.ident)
            }
            UseTree::Group(group) => {
                for tree in &group.items {
                    self.use_tree(prefix, tree);
                }
            }
            _ => {
                self.paths.insert(prefix.clone());
            }
        }
    }

    fn use_name(&mut self, prefix: &[String], ident: &syn::Ident) {
        let mut path = prefix.to_vec();
        path.push(ident.to_string());
        self.paths.insert(path);
    }
}

impl<'ast> Visit<'ast> for References {
    fn visit_visibility(&mut self, _: &'ast syn::Visibility) {
        // A scoped visibility path limits access to a declaration.
    }

    fn visit_item_use(&mut self, import: &'ast syn::ItemUse) {
        if !test_only(&import.attrs) {
            self.use_tree(&mut Vec::new(), &import.tree);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        // Single names are symbol references. Imports are collected separately above.
        if path.segments.len() > 1 {
            self.paths.insert(
                path.segments
                    .iter()
                    .map(|segment| segment.ident.to_string())
                    .collect(),
            );
        }
        syn::visit::visit_path(self, path);
    }
}

fn resolve_module(
    source: &[String],
    reference: &[String],
    modules: &BTreeMap<Module, Vec<Item>>,
) -> Option<Module> {
    let (prefix, relative) = match reference {
        [first, rest @ ..] if first == "crate" => (&[][..], rest),
        [first, rest @ ..] if first == "self" => (source, rest),
        [first, ..] if first == "super" => {
            let levels = reference.iter().take_while(|part| *part == "super").count();
            (
                source.get(..source.len().checked_sub(levels)?)?,
                &reference[levels..],
            )
        }
        _ => (source, reference),
    };

    let candidate = [prefix, relative].concat();
    (prefix.len() + 1..=candidate.len()).rev().find_map(|end| {
        modules
            .contains_key(&candidate[..end])
            .then(|| candidate[..end].to_vec())
    })
}

fn dependency_graph(modules: &BTreeMap<Module, Vec<Item>>) -> (Graph, Vec<String>) {
    let mut graph = Graph::new();
    let mut violations = Vec::new();
    for (module, items) in modules {
        let mut references = References::default();
        for item in items {
            references.visit_item(item);
        }
        let dependencies = graph.entry(module.clone()).or_default();
        let layer = module.first().map_or("main", String::as_str);
        for reference in references.paths {
            if let Some(target) = resolve_module(module, &reference, modules) {
                if target != *module {
                    if !allows_layer(layer, &target[0]) {
                        violations.push(format!(
                            "{} must not depend on {}",
                            name(module),
                            name(&target)
                        ));
                    }
                    dependencies.insert(target);
                }
            } else if forbidden_external(layer, &reference) {
                violations.push(format!(
                    "{} must not use {}",
                    name(module),
                    name(&reference)
                ));
            }
        }
    }
    (graph, violations)
}

fn allows_layer(source: &str, target: &str) -> bool {
    if source == target {
        return true;
    }
    match source {
        "main" => target == "desktop",
        "desktop" => matches!(target, "state" | "model" | "profiles"),
        "state" => matches!(target, "cli" | "profiles" | "model"),
        "cli" => matches!(target, "profiles" | "model"),
        "profiles" => target == "model",
        _ => false,
    }
}

fn forbidden_external(layer: &str, reference: &[String]) -> bool {
    let Some(first) = reference.first() else {
        return false;
    };
    if first == "tauri" || first.starts_with("tauri_plugin_") || first == "reqwest" {
        return layer != "desktop";
    }
    if reference.starts_with(&["tokio".into(), "process".into()])
        || reference.starts_with(&["std".into(), "process".into(), "Command".into()])
    {
        return layer != "cli";
    }
    layer == "model" && (first == "tokio" || reference.starts_with(&["std".into(), "fs".into()]))
}

fn find_cycle(graph: &Graph) -> Option<Vec<Module>> {
    fn walk(
        node: &Module,
        graph: &Graph,
        active: &mut Vec<Module>,
        finished: &mut BTreeSet<Module>,
    ) -> Option<Vec<Module>> {
        if let Some(start) = active.iter().position(|module| module == node) {
            return Some(
                active[start..]
                    .iter()
                    .chain(std::iter::once(node))
                    .cloned()
                    .collect(),
            );
        }
        if finished.contains(node) {
            return None;
        }
        active.push(node.clone());
        for target in &graph[node] {
            if let Some(cycle) = walk(target, graph, active, finished) {
                return Some(cycle);
            }
        }
        active.pop();
        finished.insert(node.clone());
        None
    }

    let mut finished = BTreeSet::new();
    graph
        .keys()
        .find_map(|module| walk(module, graph, &mut Vec::new(), &mut finished))
}

#[test]
fn catches_reverse_layer_edges_and_cycles() {
    let mut modules = BTreeMap::new();
    for (module, code) in [
        ("state", "use crate::{cli::Login, model::Session};"),
        ("cli", "fn verify(_: crate::state::State) {}"),
        ("model", "use tauri_plugin_dialog::DialogExt;"),
    ] {
        modules.insert(vec![module.into()], syn::parse_file(code).unwrap().items);
    }
    let (graph, violations) = dependency_graph(&modules);
    assert_eq!(violations.len(), 2, "{violations:?}");
    assert!(find_cycle(&graph).is_some());
}

#[test]
fn scoped_visibility_preserves_dependency_direction() {
    let modules = BTreeMap::from([
        (
            vec!["desktop".into()],
            syn::parse_file("use self::commands::dashboard;")
                .unwrap()
                .items,
        ),
        (
            vec!["desktop".into(), "commands".into()],
            syn::parse_file("pub(in crate::desktop) fn dashboard() {}")
                .unwrap()
                .items,
        ),
    ]);
    let (graph, violations) = dependency_graph(&modules);
    assert!(violations.is_empty(), "{violations:?}");
    assert!(find_cycle(&graph).is_none());
}
