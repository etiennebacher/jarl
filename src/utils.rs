use air_r_syntax::{RSyntaxKind, RSyntaxNode};

pub fn find_new_lines(ast: &RSyntaxNode) -> Vec<u32> {
    ast.first_child()
        .unwrap()
        .text()
        .to_string()
        .match_indices("\n")
        .map(|x| x.0.try_into().unwrap())
        .collect::<Vec<u32>>()
}

pub fn find_row_col(ast: &RSyntaxNode, loc_new_lines: &[u32]) -> (u32, u32) {
    let start: u32 = ast.text_range().start().into();
    let new_lines_before = loc_new_lines
        .iter()
        .filter(|x| *x <= &start)
        .collect::<Vec<&u32>>();
    let n_new_lines: u32 = new_lines_before.len().try_into().unwrap();
    let last_new_line = match new_lines_before.last() {
        Some(x) => **x,
        None => 0_u32,
    };
    let col: u32 = start - last_new_line + 1;
    let row: u32 = n_new_lines + 1;
    (row, col)
}

pub fn get_args(node: &RSyntaxNode) -> Option<RSyntaxNode> {
    node.descendants()
        .find(|x| x.kind() == RSyntaxKind::R_ARGUMENT)
}
