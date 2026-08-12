//! The symbol index (G8.1) and graph construction (G8.2).
//!
//! Rust source is parsed with `syn` (a real parser, not regex) so structural symbols are
//! correct. Each symbol is repository/worktree/revision-bound and carries a content hash of
//! its source slice. The indexer never follows symlinks, never ingests sensitive classes,
//! and skips binary/ignored paths.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use kern_types::{RepositoryId, WorktreeId};
use syn::spanned::Spanned;
use syn::visit::Visit;

use crate::graph::DependencyGraph;
use crate::hash::content_hash;
use crate::security::{is_binary_like, is_ignored_for_relevance, sensitive_reason};
use crate::types::{
    ContentHash, ContextError, ContextEvent, EdgeKind, FileId, Revision, Span, Symbol, SymbolId,
    SymbolKind,
};

/// A recorded source file.
#[derive(Debug, Clone)]
pub struct FileRecord {
    /// Worktree-relative path.
    pub path: PathBuf,
    /// Full source text.
    pub source: String,
    /// Line-split source (for slicing symbol snippets).
    pub lines: Vec<String>,
    /// Hash of the whole file.
    pub content_hash: ContentHash,
}

/// A parsed, repository-bound index of a worktree.
#[derive(Debug)]
pub struct SymbolIndex {
    repository: RepositoryId,
    worktree: WorktreeId,
    worktree_path: PathBuf,
    revision: Revision,
    symbols: Vec<Symbol>,
    files: BTreeMap<FileId, FileRecord>,
    graph: DependencyGraph,
    name_to_syms: BTreeMap<String, Vec<SymbolId>>,
    id_to_idx: BTreeMap<SymbolId, usize>,
    events: Vec<ContextEvent>,
}

impl SymbolIndex {
    /// The repository this index is bound to.
    #[must_use]
    pub fn repository(&self) -> &RepositoryId {
        &self.repository
    }

    /// The worktree this index is bound to.
    #[must_use]
    pub fn worktree(&self) -> &WorktreeId {
        &self.worktree
    }

    /// The canonical worktree path.
    #[must_use]
    pub fn worktree_path(&self) -> &Path {
        &self.worktree_path
    }

    /// The revision this index was built at.
    #[must_use]
    pub fn revision(&self) -> &Revision {
        &self.revision
    }

    /// All symbols.
    #[must_use]
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    /// All file records.
    #[must_use]
    pub fn files(&self) -> &BTreeMap<FileId, FileRecord> {
        &self.files
    }

    /// The dependency graph.
    #[must_use]
    pub fn graph(&self) -> &DependencyGraph {
        &self.graph
    }

    /// Symbol ids with a given simple name.
    #[must_use]
    pub fn by_name(&self, name: &str) -> &[SymbolId] {
        self.name_to_syms.get(name).map_or(&[], Vec::as_slice)
    }

    /// Look up a symbol by id.
    #[must_use]
    pub fn symbol(&self, id: &SymbolId) -> Option<&Symbol> {
        self.id_to_idx.get(id).map(|&i| &self.symbols[i])
    }

    /// The emitted (secret-free) events.
    #[must_use]
    pub fn events(&self) -> &[ContextEvent] {
        &self.events
    }

    /// Build an index by parsing every eligible file under `worktree_path`.
    pub fn build(
        repository: RepositoryId,
        worktree: WorktreeId,
        worktree_path: &Path,
        revision: Revision,
    ) -> Result<Self, ContextError> {
        let root = fs::canonicalize(worktree_path).map_err(|e| ContextError::Io(e.to_string()))?;
        let mut idx = Self {
            repository,
            worktree,
            worktree_path: root.clone(),
            revision,
            symbols: Vec::new(),
            files: BTreeMap::new(),
            graph: DependencyGraph::new(),
            name_to_syms: BTreeMap::new(),
            id_to_idx: BTreeMap::new(),
            events: vec![ContextEvent::IndexStarted],
        };

        let mut rel_files: Vec<PathBuf> = Vec::new();
        collect_files(&root, &root, &mut rel_files);
        rel_files.sort(); // deterministic order

        // Pass 1: record files and extract symbols.
        let mut pending_refs: Vec<(SymbolId, Vec<CallRef>)> = Vec::new();
        for rel in &rel_files {
            if let Some(reason) = sensitive_reason(rel) {
                idx.events
                    .push(ContextEvent::Denied(format!("{}: {reason}", rel.display())));
                continue;
            }
            let abs = root.join(rel);
            let Ok(source) = fs::read_to_string(&abs) else {
                continue; // unreadable / non-utf8 -> skip
            };
            let file_id = FileId(rel.to_string_lossy().into_owned());
            let lines: Vec<String> = source.lines().map(str::to_string).collect();
            idx.files.insert(
                file_id.clone(),
                FileRecord {
                    path: rel.clone(),
                    content_hash: ContentHash(content_hash(source.as_bytes())),
                    lines: lines.clone(),
                    source: source.clone(),
                },
            );

            if rel.extension().is_some_and(|e| e == "rs") {
                match syn::parse_file(&source) {
                    Ok(ast) => {
                        let mut ext = Extractor {
                            idx: &mut idx,
                            file_id: &file_id,
                            rel,
                            lines: &lines,
                            pending: &mut pending_refs,
                        };
                        ext.walk_items(&ast.items, &[], None);
                    }
                    Err(e) => {
                        idx.events.push(ContextEvent::Denied(format!(
                            "parse {}: {e}",
                            rel.display()
                        )));
                    }
                }
            }
        }

        // Pass 2: resolve name-based reference/call edges (unique internal name only).
        for (owner, refs) in pending_refs {
            for r in refs {
                if let Some(targets) = idx.name_to_syms.get(&r.name) {
                    if targets.len() == 1 && targets[0] != owner {
                        let kind = if r.is_call {
                            EdgeKind::Calls
                        } else {
                            EdgeKind::References
                        };
                        idx.graph.add_edge(owner.clone(), targets[0].clone(), kind);
                    }
                    // ambiguous (len>1) is left unresolved rather than invented
                }
            }
        }

        let count = idx.symbols.len();
        idx.events.push(ContextEvent::IndexCompleted(count));
        Ok(idx)
    }
}

/// A name referenced from within a symbol body, with whether it appeared in call position.
struct CallRef {
    name: String,
    is_call: bool,
}

/// Collects identifiers referenced within an item body.
#[derive(Default)]
struct RefCollector {
    refs: Vec<CallRef>,
}

impl<'ast> Visit<'ast> for RefCollector {
    fn visit_expr_call(&mut self, node: &'ast syn::ExprCall) {
        if let syn::Expr::Path(p) = &*node.func {
            if let Some(seg) = p.path.segments.last() {
                self.refs.push(CallRef {
                    name: seg.ident.to_string(),
                    is_call: true,
                });
            }
        }
        syn::visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        self.refs.push(CallRef {
            name: node.method.to_string(),
            is_call: true,
        });
        syn::visit::visit_expr_method_call(self, node);
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        if let Some(seg) = node.segments.last() {
            self.refs.push(CallRef {
                name: seg.ident.to_string(),
                is_call: false,
            });
        }
        syn::visit::visit_path(self, node);
    }
}

/// Extracts symbols from a parsed file into the index.
struct Extractor<'a> {
    idx: &'a mut SymbolIndex,
    file_id: &'a FileId,
    rel: &'a Path,
    lines: &'a [String],
    pending: &'a mut Vec<(SymbolId, Vec<CallRef>)>,
}

impl Extractor<'_> {
    fn snippet_hash(&self, span: Span) -> ContentHash {
        let start = span.start_line.saturating_sub(1);
        let end = span.end_line.min(self.lines.len());
        let slice = self.lines.get(start..end).unwrap_or(&[]).join("\n");
        ContentHash(content_hash(slice.as_bytes()))
    }

    fn make_symbol(
        &mut self,
        kind: SymbolKind,
        name: &str,
        module_path: &[String],
        span: Span,
    ) -> SymbolId {
        let qualified = if module_path.is_empty() {
            name.to_string()
        } else {
            format!("{}::{name}", module_path.join("::"))
        };
        let rel_str = self.rel.to_string_lossy();
        let raw = format!("{rel_str}|{kind:?}|{qualified}|{}", span.start_line);
        let id = SymbolId(format!("sym:{}", crate::hash::content_hash(raw.as_bytes())));
        let content_hash = self.snippet_hash(span);
        let sym = Symbol {
            id: id.clone(),
            kind,
            name: name.to_string(),
            qualified,
            file: self.file_id.clone(),
            path: self.rel.to_path_buf(),
            span,
            repository: self.idx.repository.clone(),
            worktree: self.idx.worktree.clone(),
            revision: self.idx.revision.clone(),
            content_hash,
        };
        let idx_pos = self.idx.symbols.len();
        self.idx.symbols.push(sym);
        self.idx.id_to_idx.insert(id.clone(), idx_pos);
        self.idx
            .name_to_syms
            .entry(name.to_string())
            .or_default()
            .push(id.clone());
        id
    }

    fn walk_items(
        &mut self,
        items: &[syn::Item],
        module_path: &[String],
        parent: Option<&SymbolId>,
    ) {
        for item in items {
            self.walk_item(item, module_path, parent);
        }
    }

    #[allow(clippy::too_many_lines)]
    fn walk_item(&mut self, item: &syn::Item, module_path: &[String], parent: Option<&SymbolId>) {
        let span = span_of(item.span());
        match item {
            syn::Item::Mod(m) => {
                let id =
                    self.make_symbol(SymbolKind::Module, &m.ident.to_string(), module_path, span);
                self.contains(parent, &id);
                if let Some((_, inner)) = &m.content {
                    let mut child_path = module_path.to_vec();
                    child_path.push(m.ident.to_string());
                    self.walk_items(inner, &child_path, Some(&id));
                }
            }
            syn::Item::Struct(s) => {
                let id =
                    self.make_symbol(SymbolKind::Struct, &s.ident.to_string(), module_path, span);
                self.contains(parent, &id);
            }
            syn::Item::Enum(e) => {
                let id =
                    self.make_symbol(SymbolKind::Enum, &e.ident.to_string(), module_path, span);
                self.contains(parent, &id);
            }
            syn::Item::Trait(t) => {
                let id =
                    self.make_symbol(SymbolKind::Trait, &t.ident.to_string(), module_path, span);
                self.contains(parent, &id);
            }
            syn::Item::Const(c) => {
                let id =
                    self.make_symbol(SymbolKind::Const, &c.ident.to_string(), module_path, span);
                self.contains(parent, &id);
            }
            syn::Item::Static(s) => {
                let id =
                    self.make_symbol(SymbolKind::Static, &s.ident.to_string(), module_path, span);
                self.contains(parent, &id);
            }
            syn::Item::Type(t) => {
                let id = self.make_symbol(
                    SymbolKind::TypeAlias,
                    &t.ident.to_string(),
                    module_path,
                    span,
                );
                self.contains(parent, &id);
            }
            syn::Item::Fn(f) => {
                let id = self.make_symbol(
                    SymbolKind::Function,
                    &f.sig.ident.to_string(),
                    module_path,
                    span,
                );
                self.contains(parent, &id);
                self.collect_refs(&id, |c| c.visit_item_fn(f));
            }
            syn::Item::Use(u) => {
                let name = use_leaf_name(&u.tree);
                let id = self.make_symbol(SymbolKind::Use, &name, module_path, span);
                self.contains(parent, &id);
            }
            syn::Item::Impl(im) => {
                let ty = type_name(&im.self_ty);
                let label = im.trait_.as_ref().map_or_else(
                    || format!("impl {ty}"),
                    |(_, path, _)| format!("impl {} for {ty}", path_last(path)),
                );
                let id = self.make_symbol(SymbolKind::Impl, &label, module_path, span);
                self.contains(parent, &id);
                // Implements edges to internal trait/type by unique name (resolved in pass 2
                // via references would be imprecise; record as references here).
                if let Some((_, path, _)) = &im.trait_ {
                    self.pending.push((
                        id.clone(),
                        vec![CallRef {
                            name: path_last(path),
                            is_call: false,
                        }],
                    ));
                }
                self.pending.push((
                    id.clone(),
                    vec![CallRef {
                        name: ty.clone(),
                        is_call: false,
                    }],
                ));
                for it in &im.items {
                    if let syn::ImplItem::Fn(m) = it {
                        let mspan = span_of(m.span());
                        let mut mp = module_path.to_vec();
                        mp.push(label.clone());
                        let mid = self.make_symbol(
                            SymbolKind::Method,
                            &m.sig.ident.to_string(),
                            &mp,
                            mspan,
                        );
                        self.contains(Some(&id), &mid);
                        self.collect_refs(&mid, |c| c.visit_impl_item_fn(m));
                    }
                }
            }
            _ => {}
        }
    }

    fn contains(&mut self, parent: Option<&SymbolId>, child: &SymbolId) {
        if let Some(p) = parent {
            self.idx
                .graph
                .add_edge(p.clone(), child.clone(), EdgeKind::Contains);
        }
    }

    fn collect_refs(&mut self, owner: &SymbolId, run: impl FnOnce(&mut RefCollector)) {
        let mut c = RefCollector::default();
        run(&mut c);
        if !c.refs.is_empty() {
            self.pending.push((owner.clone(), c.refs));
        }
    }
}

fn span_of(s: proc_macro2::Span) -> Span {
    Span {
        start_line: s.start().line,
        end_line: s.end().line,
    }
}

fn type_name(ty: &syn::Type) -> String {
    match ty {
        syn::Type::Path(p) => path_last(&p.path),
        syn::Type::Reference(r) => type_name(&r.elem),
        _ => "?".to_string(),
    }
}

fn path_last(path: &syn::Path) -> String {
    path.segments
        .last()
        .map_or_else(|| "?".to_string(), |s| s.ident.to_string())
}

fn use_leaf_name(tree: &syn::UseTree) -> String {
    match tree {
        syn::UseTree::Path(p) => use_leaf_name(&p.tree),
        syn::UseTree::Name(n) => n.ident.to_string(),
        syn::UseTree::Rename(r) => r.rename.to_string(),
        syn::UseTree::Glob(_) => "*".to_string(),
        syn::UseTree::Group(g) => g
            .items
            .first()
            .map_or_else(|| "?".to_string(), use_leaf_name),
    }
}

/// Recursively collect worktree-relative file paths, skipping symlinks, ignored dirs, and
/// binary-like files.
fn collect_files(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_symlink() {
            continue; // never follow symlinks during indexing
        }
        let path = entry.path();
        let Ok(rel) = path.strip_prefix(root) else {
            continue;
        };
        if is_ignored_for_relevance(rel) {
            continue;
        }
        if ft.is_dir() {
            collect_files(root, &path, out);
        } else if ft.is_file() && !is_binary_like(rel) {
            out.push(rel.to_path_buf());
        }
    }
}
