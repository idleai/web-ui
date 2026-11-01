use super::plain;

#[test]
fn recorded_wrappers_do_not_replace_the_description_or_inflate_line_counts() {
    assert_eq!(
        plain("<task>\r\n# Review `src/main.rs`\r\n\r\nSecond line\r\n</task>"),
        "Review src/main.rs · Second line",
        "meaningful lines share a single plain summary"
    );
    assert_eq!(
        plain("<b>Review</b> and <i>update</i> work_unit_id"),
        "Review and update work_unit_id",
        "wrappers disappear while identifiers retain underscores"
    );
}

#[test]
fn descriptions_keep_code_literal_and_links_non_navigable() {
    let text =
        plain("- **Review** `src/main.rs` and [file](javascript:alert)\n```\nMore details\n```");
    assert_eq!(
        text, "Review src/main.rs and file · More details",
        "links retain labels without destinations or formatting markup"
    );
    assert!(
        !text.contains("javascript:"),
        "link destinations cannot become active markup"
    );
}
