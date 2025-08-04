use crate::location::Location;
use crate::message::Diagnostic;
use air_r_syntax::{RSyntaxKind, RSyntaxNode};
use anyhow::{Result, anyhow};

pub fn find_new_lines(ast: &RSyntaxNode) -> Result<Vec<usize>> {
    match ast.first_child() {
        Some(rootnode) => Ok(rootnode
            .text()
            .to_string()
            .match_indices("\n")
            .map(|x| x.0)
            .collect::<Vec<usize>>()),
        None => Err(anyhow!(
            "Couldn't find root node. Maybe the document contains a parsing error?"
        )),
    }
}

pub fn find_row_col(start: usize, loc_new_lines: &[usize]) -> (usize, usize) {
    let new_lines_before = loc_new_lines
        .iter()
        .filter(|x| *x <= &start)
        .collect::<Vec<&usize>>();
    let n_new_lines = new_lines_before.len();
    let last_new_line = match new_lines_before.last() {
        Some(x) => **x,
        None => 0_usize,
    };

    let col: usize = start - last_new_line;
    let row: usize = n_new_lines + 1;
    (row, col)
}

pub fn compute_lints_location(
    diagnostics: Vec<Diagnostic>,
    loc_new_lines: &[usize],
) -> Vec<Diagnostic> {
    diagnostics
        .into_iter()
        .map(|mut diagnostic| {
            let start: usize = diagnostic.range.start().into();
            let loc = find_row_col(start, loc_new_lines);
            diagnostic.location = Some(Location::new(loc.0, loc.1));
            diagnostic
        })
        .collect()
}

pub fn get_first_arg(node: &RSyntaxNode) -> Option<RSyntaxNode> {
    node.descendants()
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
}

pub fn get_args(node: &RSyntaxNode) -> Vec<RSyntaxNode> {
    node.descendants()
        // Limit to first list of arguments to avoid collecting arguments from nested functions
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT_LIST)
        .unwrap()
        .descendants()
        .filter(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
        .collect::<Vec<_>>()
}

pub fn node_is_in_square_brackets(ast: &RSyntaxNode) -> bool {
    let great_grandparent = ast.ancestors().nth(3);
    match great_grandparent {
        Some(x) => x.kind() == RSyntaxKind::R_SUBSET_ARGUMENTS,
        None => false,
    }
}

#[cfg(test)]
mod tests {

    use std::fs;
    use std::process::{Command, Stdio};
    use tempfile::Builder;

    #[test]
    fn parsing_error_doesnt_panic() {
        let temp_file = Builder::new()
            .prefix("test-flir")
            .suffix(".R")
            .tempfile()
            .unwrap();

        fs::write(&temp_file, "blah = fun(1) {").expect("Failed to write initial content");

        let output = Command::new("flir")
            .arg("--dir")
            .arg(temp_file.path())
            .stdout(Stdio::piped())
            .output()
            .expect("Failed to execute command");

        let err_message = String::from_utf8_lossy(&output.stderr).to_string();
        assert!(err_message.contains("Maybe the document contains a parsing error"))
    }
}
