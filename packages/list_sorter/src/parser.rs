use std::collections::BTreeSet;

pub fn parse_lines(content: &str, case_insensitive: bool) -> BTreeSet<String> {
    content
        .lines()
        .filter_map(|line| process_line(line, case_insensitive))
        .collect()
}

fn process_line(line: &str, case_insensitive: bool) -> Option<String> {
    let line = line.trim();

    if line.is_empty() {
        return None;
    }
    let line_to_process =
        if case_insensitive { line.to_lowercase() } else { line.to_string() };

    line_to_process
        .split_once('.')
        .filter(|(prefix, _)| is_ordered_list_item(prefix))
        .map(|(_, rest)| rest.trim().to_string())
        .or_else(|| Some(line.to_string()))
}

fn is_ordered_list_item(prefix: &str) -> bool {
    prefix.chars().all(char::is_numeric)
}
