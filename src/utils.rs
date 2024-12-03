use air_r_syntax::{RSyntaxKind, RSyntaxNode};

pub fn find_new_lines(ast: &RSyntaxNode) -> Vec<usize> {
    ast.first_child()
        .unwrap()
        .text()
        .to_string()
        .match_indices("\n")
        .map(|x| x.0)
        .collect::<Vec<usize>>()
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
    let col = start - last_new_line + 1;
    let row = n_new_lines + 1;
    (row, col)
}

pub fn get_args(node: &RSyntaxNode) -> Option<RSyntaxNode> {
    node.descendants()
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
}
