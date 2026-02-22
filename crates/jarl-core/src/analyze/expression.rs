use air_r_syntax::{
    AnyRExpression, RBinaryExpressionFields, RForStatementFields, RIfStatementFields,
    RWhileStatementFields,
};

use crate::analyze;
use crate::checker::Checker;

/// Dispatch an expression to its appropriate set of rules and recurse into children.
///
/// Some expression types do both (e.g. RBinaryExpression), some only do the
/// dispatch to rules (e.g. RIdentifier), some only do the recursive call (e.g.
/// RFunctionDefinition).
///
/// Not all patterns are covered but they don't necessarily have to be.
/// For instance, there are currently no rule for RNaExpression and
/// it doesn't have any children expression on which we need to call
/// check_expression().
///
/// If a rule needs to be applied on RNaExpression in the future, then
/// we can add the corresponding match arm at this moment.
pub(crate) fn check_expression(
    expression: &air_r_syntax::AnyRExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    match expression {
        AnyRExpression::AnyRValue(children) => {
            analyze::anyvalue::anyvalue(children, checker)?;
        }
        AnyRExpression::RBinaryExpression(children) => {
            analyze::binary_expression::binary_expression(children, checker)?;
            let RBinaryExpressionFields { left, right, .. } = children.as_fields();
            check_expression(&left?, checker)?;
            check_expression(&right?, checker)?;
        }
        AnyRExpression::RBracedExpressions(children) => {
            for expr in children.expressions() {
                check_expression(&expr, checker)?;
            }
        }
        AnyRExpression::RCall(children) => {
            analyze::call::call(children, checker)?;

            if let Some(ns_expr) = children.function()?.as_r_namespace_expression() {
                analyze::namespace_expression::namespace_expression(ns_expr, checker)?;
            }

            for arg in children.arguments()?.items() {
                if let Some(expr) = arg.unwrap().as_fields().value {
                    check_expression(&expr, checker)?;
                }
            }
        }
        AnyRExpression::RForStatement(children) => {
            analyze::for_loop::for_loop(children, checker)?;
            let RForStatementFields { variable, sequence, body, .. } = children.as_fields();
            analyze::identifier::identifier(&variable?, checker)?;

            check_expression(&sequence?, checker)?;
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RFunctionDefinition(children) => {
            analyze::function_definition::function_definition(children, checker)?;
            let params = children.parameters()?.items();
            for param in params {
                let default = param?.default();
                if let Some(default) = default
                    && let Ok(default) = default.value()
                {
                    check_expression(&default, checker)?;
                }
            }
            check_expression(&children.body()?, checker)?;
        }
        AnyRExpression::RIdentifier(x) => {
            analyze::identifier::identifier(x, checker)?;
        }
        AnyRExpression::RIfStatement(children) => {
            analyze::if_::if_(children, checker)?;

            let RIfStatementFields { condition, consequence, else_clause, .. } =
                children.as_fields();
            check_expression(&condition?, checker)?;
            check_expression(&consequence?, checker)?;
            if let Some(else_clause) = else_clause {
                let alternative = else_clause.alternative();
                check_expression(&alternative?, checker)?;
            }
        }
        AnyRExpression::RNamespaceExpression(children) => {
            analyze::namespace_expression::namespace_expression(children, checker)?;
        }
        AnyRExpression::RParenthesizedExpression(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RRepeatStatement(children) => {
            let body = children.body();
            check_expression(&body?, checker)?;
        }
        AnyRExpression::RSubset(children) => {
            analyze::subset::subset(children, checker)?;

            for arg in children.arguments()?.items() {
                if let Some(expr) = arg?.value() {
                    check_expression(&expr, checker)?;
                }
            }
        }
        AnyRExpression::RSubset2(children) => {
            for arg in children.arguments()?.items() {
                if let Some(expr) = arg?.value() {
                    check_expression(&expr, checker)?;
                }
            }
        }
        AnyRExpression::RUnaryExpression(children) => {
            analyze::unary_expression::unary_expression(children, checker)?;

            let argument = children.argument();
            check_expression(&argument?, checker)?;
        }
        AnyRExpression::RWhileStatement(children) => {
            analyze::while_::while_(children, checker)?;
            let RWhileStatementFields { condition, body, .. } = children.as_fields();
            check_expression(&condition?, checker)?;
            check_expression(&body?, checker)?;
        }
        _ => {}
    }

    Ok(())
}
