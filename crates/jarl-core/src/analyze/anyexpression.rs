use crate::check::Checker;
use air_r_syntax::AnyRExpression;

pub fn anyexpression(_r_expr: &AnyRExpression, _checker: &mut Checker) -> anyhow::Result<()> {
    // blanket_suppression is now handled at file level in get_checks()
    Ok(())
}
