use crate::lints::ALL_RULES;
use air_r_syntax::{RSyntaxKind, RSyntaxNode};
use anyhow::{anyhow, Result};

pub fn find_new_lines(ast: &RSyntaxNode) -> Result<Vec<usize>> {
    match ast.first_child() {
        Some(rootnode) => Ok(rootnode
            .text()
            .to_string()
            .match_indices("\n")
            .map(|x| x.0)
            .collect::<Vec<usize>>()),
        None => Err(anyhow!("Couldn't find root node")),
    }
}

pub fn find_row_col(ast: &RSyntaxNode, loc_new_lines: &[usize]) -> (usize, usize) {
    let start: usize = ast.text_range().start().into();
    let new_lines_before = loc_new_lines
        .iter()
        .filter(|x| *x <= &start)
        .collect::<Vec<&usize>>();
    let n_new_lines = new_lines_before.len();
    let last_new_line = match new_lines_before.last() {
        Some(x) => **x,
        None => 0_usize,
    };
    let col: usize = start - last_new_line + 1;
    let row: usize = n_new_lines + 1;
    (row, col)
}

pub fn get_args(node: &RSyntaxNode) -> Option<RSyntaxNode> {
    node.descendants()
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
}

pub fn node_is_in_square_brackets(ast: &RSyntaxNode) -> bool {
    let great_grandparent = ast.ancestors().nth(3);
    match great_grandparent {
        Some(x) => x.kind() == RSyntaxKind::R_SUBSET_ARGUMENTS,
        None => false,
    }
}

pub fn parse_rules(rules: &String) -> Vec<&str> {
    if rules.as_str() == "" {
        ALL_RULES.to_vec()
    } else {
        rules.split(",").collect::<Vec<&str>>()
    }
}
