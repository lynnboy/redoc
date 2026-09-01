use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use redoc_core::ast::{ElementKind, Node};
use redoc_core::parser::parse;
use redoc_core::validate::{self, Severity};
use redoc_core::Span;

const HELP: &str = "\
redoc — tools for the redoc markup language

Usage:
  redoc parse <file>             parse a file and print the document tree
  redoc check <file|dir>         parse and validate; report diagnostics
  redoc --help                   show this help

Exit codes:
  0  success (check: no errors)
  1  parse/validation errors found
  2  usage error
";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [cmd, path] if cmd == "parse" => cmd_parse(Path::new(path)),
        [cmd, path] if cmd == "check" => cmd_check(Path::new(path)),
        [cmd] if cmd == "--help" || cmd == "-h" => {
            print!("{HELP}");
            ExitCode::SUCCESS
        }
        _ => {
            eprint!("{HELP}");
            ExitCode::from(2)
        }
    }
}

fn cmd_parse(path: &Path) -> ExitCode {
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: cannot read {}: {e}", path.display());
            return ExitCode::from(2);
        }
    };
    let doc = parse(&src);
    for n in &doc.nodes {
        print_node(n, 0, &src);
    }
    for e in &doc.errors {
        eprintln!("error: {} at byte {}", e.message, e.span.start);
    }
    if doc.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn cmd_check(path: &Path) -> ExitCode {
    let mut files = Vec::new();
    collect(path, &mut files);
    if files.is_empty() {
        eprintln!("error: no .redoc files found under {}", path.display());
        return ExitCode::from(2);
    }
    files.sort();

    // Phase 1: parse each file and discard it, collecting only the anchor
    // index. Holding the full parse trees of the entire corpus is what
    // bloats memory; the index alone is small.
    let mut all_anchors: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut total_parse_errors = 0usize;
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", f.display());
                total_parse_errors += 1;
                continue;
            }
        };
        let doc = parse(&src);
        total_parse_errors += doc.errors.len();
        for e in &doc.errors {
            report(f, "error", &e.message, e.span, &src);
        }
        collect_anchors(&doc.nodes, &mut all_anchors);
        drop(doc);
        drop(src);
    }

    // Phase 2: re-read each file, validate, report, discard. Only one file's
    // tree is ever live at a time.
    let mut total_errors = total_parse_errors;
    let mut total_warnings = 0usize;
    for f in &files {
        let src = match std::fs::read_to_string(f) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let doc = parse(&src);
        let v = validate::validate(&doc.nodes);
        for d in &v.diagnostics {
            // A reference whose anchor lives in another file is fine; only
            // report it if it is missing from the corpus-wide index.
            let is_unresolved_ref = d.message.starts_with("unresolved reference");
            if is_unresolved_ref {
                let id = d
                    .message
                    .trim_start_matches("unresolved reference `")
                    .trim_end_matches('`');
                if all_anchors.contains(id) {
                    continue;
                }
            }
            match d.severity {
                Severity::Error => {
                    report(f, "error", &d.message, d.span, &src);
                    total_errors += 1;
                }
                Severity::Warning => {
                    report(f, "warning", &d.message, d.span, &src);
                    total_warnings += 1;
                }
            }
        }
        drop(doc);
        drop(src);
    }

    println!(
        "checked {} files: {total_errors} errors, {total_warnings} warnings",
        files.len()
    );
    if total_errors == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    if dir.is_file() {
        if dir.extension().is_some_and(|e| e == "redoc") {
            out.push(dir.to_path_buf());
        }
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else {
        eprintln!("error: cannot read directory {}", dir.display());
        return;
    };
    for ent in rd.flatten() {
        let p = ent.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p.extension().is_some_and(|e| e == "redoc") {
            out.push(p);
        }
    }
}

fn collect_anchors(nodes: &[Node], out: &mut std::collections::HashSet<String>) {
    for n in nodes {
        if let Node::Element(e) = n {
            if let Some(id) = &e.anchor {
                out.insert(id.clone());
            }
            collect_anchors(&e.content, out);
            collect_anchors(&e.body, out);
        }
    }
}

fn report(file: &Path, kind: &str, msg: &str, span: Span, src: &str) {
    let (line, col) = line_col(src, span.start);
    let mut out = std::io::stderr().lock();
    let _ = writeln!(
        out,
        "{}:{}:{}: {kind}: {msg}",
        file.display(),
        line + 1,
        col + 1
    );
    // Show the offending line.
    if let Some(l) = src.lines().nth(line) {
        let _ = writeln!(out, "  {}", l.trim());
        let _ = writeln!(out, "  {}^", " ".repeat(col));
    }
}

fn line_col(src: &str, byte: usize) -> (usize, usize) {
    let mut line = 0usize;
    let mut col = 0usize;
    for (i, c) in src.char_indices() {
        if i >= byte {
            break;
        }
        if c == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }
    (line, col)
}

fn print_node(n: &Node, indent: usize, src: &str) {
    match n {
        Node::Text { text, .. } => {
            let t = text.trim();
            if !t.is_empty() {
                println!("{}{}", "  ".repeat(indent), t);
            }
        }
        Node::Comment { .. } => {}
        Node::Element(e) => {
            println!(
                "{}{} {}{}",
                "  ".repeat(indent),
                e.tag_name(),
                e.anchor.as_deref().unwrap_or(""),
                if e.kind == ElementKind::SubLanguage {
                    format!(
                        " <{} raw bytes>",
                        e.raw.as_deref().map_or(0, |r| r.len())
                    )
                } else {
                    String::new()
                }
            );
            for c in &e.content {
                print_node(c, indent + 1, src);
            }
            for b in &e.body {
                print_node(b, indent + 1, src);
            }
        }
    }
}
