use crate::ast::{ElementKind, Node};
use crate::span::Span;

/// Severity of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Warning,
    Error,
}

/// A semantic problem found while validating a document.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Span,
}

/// The result of validating a document.
#[derive(Debug, Default)]
pub struct Validation {
    pub diagnostics: Vec<Diagnostic>,
}

/// Anchor definitions collected from a document: `[section#id]`,
/// `[item#id]`, `[table#id]`, `[formula#id]`, `[?impldef#id]`, ...
#[derive(Debug, Clone)]
pub struct Anchor {
    pub id: String,
    pub tag: String,
    pub span: Span,
}

/// Reference sites: `[#id]`, `[#:note id]`, ...
#[derive(Debug, Clone)]
pub struct Reference {
    pub id: String,
    pub span: Span,
}

/// Well-known tag names. Unknown names produce a warning, since the format
/// keeps evolving.
const KNOWN_TAGS: &[&str] = &[
    // blocks
    "document",
    "section",
    "para",
    "div",
    "list",
    "item",
    "rule",
    "syntax",
    "codeblock",
    "table",
    "formula",
    "math",
    "note",
    "begin",
    "end",
    "include",
    "attribute",
    "value",
    "br",
    "url",
    "cite",
    // marker tags
    "~",
    "*",
    "^",
    "$",
    "=",
    "#",
    "+",
    "%",
    "!",
    "&",
    "\"",
    "|",
    "-",
    "`",
    // eval tags
    "libheader",
    "libhreader",
    "impldef",
    "impdefx",
    "impldefrootname",
    "ubdef",
    "ub",
    "ifndr",
    "see",
    "also",
    "bigoh",
    "rationale",
    "effect",
    "change",
    "difficulty",
    "howwide",
    "termref",
    "ref",
    "xrefc",
    "indexordmem",
    "indexunordmem",
    "indexcont",
    "indexcond",
    "replaceabledesc",
    "unseenspec",
    "specterm",
];

const CODEBLOCK_SUBTYPES: &[&str] = &["declaration", "synopsis", "notation", "literal", "output"];

/// Walk a document, collecting anchors and references and checking
/// structural invariants that the parser cannot see.
pub fn validate(doc: &[Node]) -> Validation {
    let mut v = Validation::default();
    let mut anchors: Vec<Anchor> = Vec::new();
    let mut refs: Vec<Reference> = Vec::new();
    let mut en_count = 0usize;
    let mut zh_count = 0usize;

    walk(doc, &mut |node| {
        match node {
            Node::Element(e) => {
                if let Some(id) = &e.anchor {
                    anchors.push(Anchor {
                        id: id.clone(),
                        tag: e.tag_name(),
                        span: e.span,
                    });
                }
                match e.name.as_str() {
                    "#" => {
                        let id = text_content(&e.content);
                        refs.push(Reference { id, span: e.span });
                    }
                    "" => match e.subtype.as_deref() {
                        Some("en") => en_count += 1,
                        Some("zh_CN") => zh_count += 1,
                        _ => {}
                    },
                    "codeblock" => {
                        if let Some(s) = &e.subtype {
                            if !CODEBLOCK_SUBTYPES.contains(&s.as_str()) {
                                v.diagnostics.push(Diagnostic {
                                    severity: Severity::Warning,
                                    message: format!(
                                        "unknown codeblock subtype `:{s}` (expected one of {})",
                                        CODEBLOCK_SUBTYPES.join(", ")
                                    ),
                                    span: e.span,
                                });
                            }
                        }
                    }
                    _ => {}
                }
                if !e.name.is_empty() && !KNOWN_TAGS.contains(&e.name.as_str()) {
                    v.diagnostics.push(Diagnostic {
                        severity: Severity::Warning,
                        message: format!("unknown tag name `{}`", e.name),
                        span: e.span,
                    });
                }
            }
            _ => {}
        }
    });

    // Duplicate anchors.
    let mut seen: std::collections::HashMap<&str, &Span> = std::collections::HashMap::new();
    for a in &anchors {
        if let Some(prev) = seen.insert(&a.id, &a.span) {
            v.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!("duplicate anchor `{}` (first at byte {})", a.id, prev.start),
                span: a.span,
            });
        }
    }

    // Unresolved references.
    for r in &refs {
        if r.id.is_empty() {
            continue;
        }
        if !anchors.iter().any(|a| a.id == r.id) {
            v.diagnostics.push(Diagnostic {
                severity: Severity::Error,
                message: format!("unresolved reference `{}`", r.id),
                span: r.span,
            });
        }
    }

    // Bilingual pairing.
    if en_count != zh_count {
        v.diagnostics.push(Diagnostic {
            severity: Severity::Warning,
            message: format!("bilingual imbalance: {en_count} [:en] vs {zh_count} [:zh_CN]"),
            span: Span::new(0, 0),
        });
    }

    v
}

/// Concatenated text content of a node list (used for `[#id]`).
fn text_content(nodes: &[Node]) -> String {
    let mut s = String::new();
    for n in nodes {
        match n {
            Node::Text { text, .. } => s.push_str(text),
            Node::Element(e) => s.push_str(&text_content(&e.content)),
            Node::Comment { .. } => {}
        }
    }
    s.trim().to_string()
}

fn walk<'a>(nodes: &'a [Node], f: &mut impl FnMut(&'a Node)) {
    for n in nodes {
        f(n);
        if let Node::Element(e) = n {
            walk(&e.content, f);
            walk(&e.body, f);
            if e.kind == ElementKind::SubLanguage {
                // Raw sub-language content: scan for `// [:en] ... [:zh_CN]`
                // pairs and `[[redoc("...")]]` embeddings later.
            }
        }
    }
}

/// Extract `[include name]` targets from a document.
pub fn includes(doc: &[Node]) -> Vec<String> {
    let mut out = Vec::new();
    walk(doc, &mut |n| {
        if let Node::Element(e) = n {
            if e.name == "include" {
                let name = text_content(&e.content);
                if !name.is_empty() {
                    out.push(name);
                }
            }
        }
    });
    out
}
