use super::{
    markdown::{MdInline, MdLineKind, Summary},
    preview,
};

#[test]
fn recorded_wrappers_do_not_replace_the_description_or_inflate_line_counts() {
    let summary = Summary::parse("<task>\r\n# Review `src/main.rs`\r\n\r\nSecond line\r\n</task>");
    assert_eq!(summary.more, 1);
    assert_eq!(summary.line.kind, MdLineKind::Heading { level: 1 });
    assert!(
        summary
            .line
            .inline
            .contains(&MdInline::Code("src/main.rs".into()))
    );
    let (element, _) = preview("<b>Review</b> and <i>update</i> work_unit_id");
    let rendered = dioxus_ssr::render_element(element);
    assert!(rendered.contains("Review and update work_unit_id"));
    assert!(!rendered.contains("<b>"));
}

#[test]
fn descriptions_keep_code_literal_and_links_non_navigable() {
    let (element, extra) =
        preview("- **Review** `<task>` and [file](javascript:alert)\n```\nMore details\n```");
    let rendered = dioxus_ssr::render_element(element);
    assert_eq!(extra, 1);
    assert!(rendered.contains("<strong>Review</strong>"));
    assert!(
        rendered.contains("<code>&#60;task&#62;</code>"),
        "{rendered}"
    );
    assert!(!rendered.contains("href="));
    assert!(!rendered.contains("<task>"));
}
