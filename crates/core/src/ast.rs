use crate::span::Span;

/// A node in the redoc document tree.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Plain text.
    Text { text: String, span: Span },
    /// A comment `[/ ... /]`.
    Comment { text: String, span: Span },
    /// Any element: a tag with its content.
    Element(Element),
}

/// How an element's body is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementKind {
    /// A region with an explicit end marker; body is collected between the
    /// opening tag and `[name:end]` / `[end:name]`.
    Region,
    /// A sub-language region (codeblock/math/formula); body is captured raw.
    SubLanguage,
    /// A leaf tag; content is only the bracket-internal part.
    Leaf,
}

/// An attribute of the form `@name` or `@name=value`.
#[derive(Debug, Clone, PartialEq)]
pub struct Attr {
    pub name: String,
    pub value: Option<String>,
}

/// A parsed tag: `[name:subtype @attr=value #anchor content]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub name: String,
    pub subtype: Option<String>,
    pub attrs: Vec<Attr>,
    pub anchor: Option<String>,
    /// Content inside the opening bracket.
    pub content: Vec<Node>,
    /// Sibling body between the opening tag and its end marker (Region only).
    pub body: Vec<Node>,
    /// Raw sub-language content (SubLanguage only).
    pub raw: Option<String>,
    pub kind: ElementKind,
    /// Span of the opening tag.
    pub span: Span,
    /// Span of the end marker, if any.
    pub end_span: Option<Span>,
}

impl Element {
    /// The full tag name including subtype, e.g. `section:chapter`, `:en`, `|`.
    pub fn tag_name(&self) -> String {
        match &self.subtype {
            Some(s) if self.name.is_empty() => format!(":{s}"),
            Some(s) => format!("{}:{s}", self.name),
            None => self.name.clone(),
        }
    }
}
