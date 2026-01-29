use crate::error::ParseError;
use crate::rule_set::Rule;
use crate::suppression::SuppressionManager;
use crate::vcs::check_version_control;
use air_fs::relativize_path;
use air_r_parser::RParserOptions;
use air_r_syntax::{
    AnyRExpression, RBinaryExpressionFields, RForStatementFields, RIfStatementFields, RSyntaxKind,
    RWhileStatementFields,
};
use anyhow::{Context, Result};
use biome_rowan::AstNode;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

use crate::analyze;
use crate::config::Config;
use crate::diagnostic::*;
use crate::fix::*;
use crate::rule_set::RuleSet;
use crate::utils::*;

pub fn check(config: Config) -> Vec<(String, Result<Vec<Diagnostic>, anyhow::Error>)> {
    // Ensure that all paths are covered by VCS. This is conservative because
    // technically we could apply fixes on those that are covered by VCS and
    // error for the others, but I'd rather be on the safe side and force the
    // user to deal with that before applying any fixes.
    if (config.apply_fixes || config.apply_unsafe_fixes) && !config.paths.is_empty() {
        let path_strings: Vec<String> = config.paths.iter().map(relativize_path).collect();
        if let Err(e) = check_version_control(&path_strings, &config) {
            let first_path = path_strings.first().unwrap().clone();
            return vec![(first_path, Err(e))];
        }
    }

    // Wrap config in Arc to avoid expensive clones in parallel execution
    let config = Arc::new(config);

    config
        .paths
        .par_iter()
        .map(|file| {
            let res = check_path(file, Arc::clone(&config));
            (relativize_path(file), res)
        })
        .collect()
}

pub fn check_path(path: &PathBuf, config: Arc<Config>) -> Result<Vec<Diagnostic>, anyhow::Error> {
    if config.apply_fixes || config.apply_unsafe_fixes {
        lint_fix(path, config)
    } else {
        lint_only(path, config)
    }
}

pub fn lint_only(path: &PathBuf, config: Arc<Config>) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let path = relativize_path(path);
    let contents = fs::read_to_string(Path::new(&path))
        .with_context(|| format!("Failed to read file: {path}"))?;

    let checks = get_checks(&contents, &PathBuf::from(&path), &config)
        .with_context(|| format!("Failed to get checks for file: {path}"))?;

    Ok(checks)
}

pub fn lint_fix(path: &PathBuf, config: Arc<Config>) -> Result<Vec<Diagnostic>, anyhow::Error> {
    let path = relativize_path(path);

    let mut has_skipped_fixes = true;
    let mut checks: Vec<Diagnostic>;

    loop {
        let contents = fs::read_to_string(Path::new(&path))
            .with_context(|| format!("Failed to read file: {path}",))?;

        checks = get_checks(&contents, &PathBuf::from(&path), &config)
            .with_context(|| format!("Failed to get checks for file: {path}",))?;

        if !has_skipped_fixes {
            break;
        }

        let (new_has_skipped_fixes, fixed_text) = apply_fixes(&checks, &contents);
        has_skipped_fixes = new_has_skipped_fixes;

        fs::write(&path, fixed_text).with_context(|| format!("Failed to write file: {path}",))?;
    }

    Ok(checks)
}

#[derive(Debug)]
// The object that will collect diagnostics in check_expressions(). One per
// analyzed file.
pub struct Checker {
    // The diagnostics to report (possibly empty).
    pub diagnostics: Vec<Diagnostic>,
    // A set of rules to apply. Each rule contains metadata about whether it
    // has a safe fix, unsafe fix, or no fix, and the minimum R version required.
    pub rule_set: RuleSet,
    // The R version that is manually passed by the user in the CLI. Any rule
    // that has a minimum R version higher than this value will be deactivated.
    pub minimum_r_version: Option<(u32, u32, u32)>,
    // Tracks comment-based suppression directives like `# jarl-ignore`
    pub suppression: SuppressionManager,
    // Which assignment operator is preferred?
    pub assignment: RSyntaxKind,
}

impl Checker {
    fn new(suppression: SuppressionManager, assignment: RSyntaxKind) -> Self {
        Self {
            diagnostics: vec![],
            rule_set: RuleSet::empty(),
            minimum_r_version: None,
            suppression,
            assignment,
        }
    }

    // This takes an Option<Diagnostic> because each lint rule reports a
    // Some(Diagnostic) or None.
    pub(crate) fn report_diagnostic(&mut self, diagnostic: Option<Diagnostic>) {
        if let Some(diagnostic) = diagnostic {
            self.diagnostics.push(diagnostic);
        }
    }

    pub(crate) fn is_rule_enabled(&mut self, rule: Rule) -> bool {
        self.rule_set.contains(&rule)
    }

    /// Get all suppressed rules for a node in a single check.
    ///
    /// Returns:
    /// - An empty set if no rules are suppressed
    /// - A set containing specific suppressed rules otherwise
    ///
    /// This combines file-level, region-level, inherited (from ancestors),
    /// and node-level suppressions.
    pub(crate) fn get_suppressed_rules(&self, node: &air_r_syntax::RSyntaxNode) -> HashSet<Rule> {
        // Fast path: if there are no suppressions anywhere, return empty set immediately
        if !self.suppression.has_any_suppressions {
            return HashSet::new();
        }

        let mut suppressed = HashSet::new();

        // Add file-level suppressions
        for rule in &self.suppression.file_suppressions {
            if self.rule_set.contains(rule) {
                suppressed.insert(*rule);
            }
        }

        // Add region-level suppressions
        let node_range = node.text_trimmed_range();
        for region in &self.suppression.skip_regions {
            if region.range.contains_range(node_range) && self.rule_set.contains(&region.rule) {
                suppressed.insert(region.rule);
            }
        }

        // Add inherited suppressions from ancestors (accumulated during traversal)
        for rule in self.suppression.inherited_suppressions.iter() {
            suppressed.insert(*rule);
        }

        // Add node-level suppressions (only this node's comments, not ancestors)
        for rule in self.suppression.check_node_comments(node) {
            if self.rule_set.contains(&rule) {
                suppressed.insert(rule);
            }
        }

        suppressed
    }
}

// Takes the R code as a string, parses it, and obtains a (possibly empty)
// vector of `Diagnostic`s.
//
// If there are diagnostics to report, this is also where their range in the
// string is converted to their location (row, column).
pub fn get_checks(contents: &str, file: &Path, config: &Config) -> Result<Vec<Diagnostic>> {
    let parser_options = RParserOptions::default();
    let parsed = air_r_parser::parse(contents, parser_options);

    if parsed.has_error() {
        return Err(ParseError { filename: file.to_path_buf() }.into());
    }

    let syntax = &parsed.syntax();
    let expressions = &parsed.tree().expressions();

    let suppression = SuppressionManager::from_node(syntax, contents);

    let mut checker = Checker::new(suppression, config.assignment);
    checker.rule_set = config.rules_to_apply.clone();
    checker.minimum_r_version = config.minimum_r_version;
    for expr in expressions {
        check_expression(&expr, &mut checker)?;
    }

    // Some rules have a fix available in their implementation but do not have
    // fix in the config, for instance because they are part of the "unfixable"
    // arg or not part of the "fixable" arg in `jarl.toml`.
    // When we get all the diagnostics with check_expression() above, we don't
    // pay attention to whether the user wants to fix them or not. Adding this
    // step here is a way to filter those fixes out before calling apply_fixes().
    let rules_without_fix = checker
        .rule_set
        .iter()
        .filter(|x| x.has_no_fix())
        .map(|x| x.name().to_string())
        .collect::<Vec<String>>();

    let diagnostics: Vec<Diagnostic> = checker
        .diagnostics
        .into_iter()
        .map(|mut x| {
            x.filename = file.to_path_buf();
            // Check if fix should be skipped based on fixable/unfixable settings
            if rules_without_fix.contains(&x.message.name) {
                x.fix = Fix::empty();
            }
            // Also check against unfixable set from config
            if config.unfixable.contains(&x.message.name) {
                x.fix = Fix::empty();
            }
            // If fixable is specified, only allow those rules to have fixes
            if let Some(ref fixable_set) = config.fixable
                && !fixable_set.contains(&x.message.name)
            {
                x.fix = Fix::empty();
            }
            // TODO: this should be removed once comments in nodes are better
            // handled, #95
            if x.fix.to_skip {
                x.fix = Fix::empty();
            }
            x
        })
        .collect();

    let loc_new_lines = find_new_lines(syntax)?;
    let diagnostics = compute_lints_location(diagnostics, &loc_new_lines);

    Ok(diagnostics)
}

/// This function updates the checker about the suppression comments it carries,
/// and calls `check_expression_inner()` to run the various rules on the node.
///
/// Updating the checker:
///
/// For each node we encounter, we already carry the suppression comments of its
/// ancestors. When we arrive at a new node, we want to:
/// 1) update the suppression comments with those of this node (if any),
/// 2) check this node (which requires checking the suppression comments to
///    know which violations to skip),
/// 3) remove the suppression comments of this node so that they are not used
///    anymore when we go to this node's siblings.
///
/// For instance, if we have:
///
/// ```r,ignore
/// # jarl-ignore assignment: reason 1
/// x <- function(x) {
///   if (x) {
///     # jarl-ignore any_is_na: reason 2
///     any(is.na(y))
///   }
///   any(is.na(x + 1))
/// }
/// ```
///
/// When we arrive at `any(is.na(y))`, we want to use both suppression comments
/// because one is attached directly to the node and the other one is attached
/// to an ancestor. However, `any(is.na(x + 1))` should only use the first
/// suppression comment, meaning that after we process `any(is.na(y))`, we need
/// to remove its suppression comment from the stack of suppression comments.
pub fn check_expression(
    expression: &air_r_syntax::AnyRExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    // Update inherited suppressions for cascading behavior.
    // We push node-level suppressions onto the stack and truncate after
    // processing children. No cloning required.
    let node = expression.syntax();
    let node_suppressions = checker.suppression.check_node_comments(node);

    // Remember stack length before adding, so we can truncate later
    let stack_len = checker.suppression.inherited_suppressions.len();
    for rule in node_suppressions {
        // Only add if enabled (filter early to keep stack small)
        if checker.rule_set.contains(&rule) {
            checker.suppression.inherited_suppressions.push(rule);
        }
    }

    let result = check_expression_inner(expression, checker);

    // Restore stack to previous length (removes what we added)
    checker
        .suppression
        .inherited_suppressions
        .truncate(stack_len);

    result
}

// This function does two things:
// - dispatch an expression to its appropriate set of rules, e.g. binary
//   expressions are sent to the rules stored in
//   analyze::binary_expression::binary_expression.
// - apply the function recursively to the expression's children (if any, which
//   is not guaranteed, e.g. for RIdentifier).
//
// Some expression types do both (e.g. RBinaryExpression), some only do the
// dispatch to rules (e.g. RIdentifier), some only do the recursive call (e.g.
// RFunctionDefinition).
//
// Not all patterns are covered but they don't necessarily have to be.
// For instance, there are currently no rule for RNaExpression and
// it doesn't have any children expression on which we need to call
// check_expression().
//
// If a rule needs to be applied on RNaExpression in the future, then
// we can add the corresponding match arm at this moment.
fn check_expression_inner(
    expression: &air_r_syntax::AnyRExpression,
    checker: &mut Checker,
) -> anyhow::Result<()> {
    analyze::anyexpression::anyexpression(expression, checker)?;
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
