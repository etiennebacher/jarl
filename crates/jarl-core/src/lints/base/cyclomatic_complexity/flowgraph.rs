//! A control-flow graph modelled on the one `cyclocomp::flowgraph()` builds, so
//! that scores are the ones of `cyclocomp::cyclocomp()` (the engine of
//! `lintr::cyclocomp_linter()`), except where cyclocomp is wrong:
//!
//! * `&&` and `||` short-circuit: a condition has separate "true" and "false"
//!   exits, so that a chain of `n` operators adds `n` paths and not `n^2 / 2`
//!   (cyclocomp #26);
//! * `switch()` has one path per arm (cyclocomp #13).
//!
//! The graph is built over a simplified view of R's language objects, [`Lang`].
//! Nodes are identified by dotted paths ("1", "1.2", "1.2.1", ...) exactly like
//! in cyclocomp, including its quirks: `is_child()` is a plain prefix test (so
//! "1.10" is a child of "1.1"), and `break` / `return` outside of a loop /
//! function make the whole computation fail.

use std::collections::{HashMap, HashSet};

/// What cyclocomp sees of an R expression.
#[derive(Debug, Clone)]
pub enum Lang {
    /// Any expression that isn't a control-flow construct (a call, an operator,
    /// a subset, etc.), as the expressions it holds in order. It adds no path
    /// by itself. Code that adds nothing (a nested function, which is scored on its
    /// own, or code that couldn't be read) is an empty `Call`.
    Call(Vec<Lang>),
    /// A call to `return`, with the same layout as `Call`.
    Return(Vec<Lang>),
    If {
        condition: Box<Lang>,
        consequence: Box<Lang>,
        alternative: Option<Box<Lang>>,
    },
    For {
        sequence: Box<Lang>,
        body: Box<Lang>,
    },
    While {
        condition: Box<Lang>,
        body: Box<Lang>,
    },
    Repeat(Box<Lang>),
    Break,
    Next,
    /// `&&` (`is_and`) and `||`.
    AndOr {
        is_and: bool,
        left: Box<Lang>,
        right: Box<Lang>,
    },
    /// `switch(selector, arms...)`. An empty arm (`a = ,`) falls through to the
    /// next one and is `None`.
    Switch {
        selector: Box<Lang>,
        arms: Vec<Option<Lang>>,
        has_default: bool,
    },
}

/// The node of the function: `return` leaves from it.
const FUNCTION: &str = "1.1";

/// Cyclomatic complexity of the body of a function, or `None` when cyclocomp
/// errors on it (`break` outside of a loop, `return` outside of a function). The
/// formals are left out: they add as many nodes as edges.
pub fn cyclocomp(body: &Lang) -> Option<usize> {
    let mut graph = Graph::default();
    graph.set_last("2", vec![]);
    graph.add_edge("1", "2");

    // The top-level object is an `expression` vector holding the function.
    graph.set_last("1", vec![FUNCTION.to_string()]);
    graph.add_edge("1", FUNCTION);
    graph.set_last(FUNCTION, vec![child(FUNCTION, 1)]);
    graph.add_edge(FUNCTION, &child(FUNCTION, 1));
    graph.walk(body, &child(FUNCTION, 1))?;

    let edges = graph.resolve_edges();
    Some((edges + 2).saturating_sub(graph.last.len()))
}

#[derive(Default)]
struct Graph {
    /// The `last` of each node: the nodes that control leaves from.
    last: HashMap<String, Vec<String>>,
    /// For `&&` and `||` nodes, the nodes control leaves from when the
    /// condition is true / false. Any other node leaves from itself.
    exits: HashMap<String, (Vec<String>, Vec<String>)>,
    edges: Vec<(String, String)>,
    breaks: Vec<String>,
    nexts: Vec<String>,
}

fn child(id: &str, i: usize) -> String {
    format!("{id}.{i}")
}

/// cyclocomp's `is_child()`: a prefix test, not a path test.
fn is_child(child: &str, parent: &str) -> bool {
    child.starts_with(parent) && child != parent
}

impl Graph {
    /// Adds a node, or replaces the `last` of an existing one.
    fn set_last(&mut self, id: &str, last: Vec<String>) {
        self.last.insert(id.to_string(), last);
    }

    fn add_edge(&mut self, from: &str, to: &str) {
        self.edges.push((from.to_string(), to.to_string()));
    }

    fn add_edges_from(&mut self, from: &[String], to: &str) {
        for from in from {
            self.add_edge(from, to);
        }
    }

    fn add_to_last(&mut self, node: &str, id: &str) -> Option<()> {
        self.last.get_mut(node)?.push(id.to_string());
        Some(())
    }

    /// The nodes control leaves from when the condition `id` is true, and false.
    fn exits(&self, id: &str) -> (Vec<String>, Vec<String>) {
        self.exits
            .get(id)
            .cloned()
            .unwrap_or_else(|| (vec![id.to_string()], vec![id.to_string()]))
    }

    fn walk(&mut self, x: &Lang, id: &str) -> Option<()> {
        match x {
            Lang::Return(elements) => {
                self.add_to_last(FUNCTION, id)?;
                self.walk_list(elements, id)
            }
            Lang::For { sequence, body } => {
                let (id1, id2) = (child(id, 1), child(id, 2));
                self.set_last(id, vec![id1.clone(), id2.clone()]);
                self.add_edge(id, &id1);
                self.add_edge(&id1, &id2);
                self.add_edge(&id2, &id2);
                self.walk(sequence, &id1)?;
                self.walk_loop(body, id, &id2, &id2)
            }
            Lang::While { condition, body } => {
                let (id1, id2) = (child(id, 1), child(id, 2));
                // `break`s are added to the `last` of this node while walking
                // the body, so it has to exist before.
                self.set_last(id, vec![]);
                self.add_edge(id, &id1);
                self.walk(condition, &id1)?;
                let (true_exits, mut last) = self.exits(&id1);
                last.push(id2.clone());
                self.set_last(id, last);

                self.add_edges_from(&true_exits, &id2);
                self.add_edge(&id2, &id1);

                self.walk_loop(body, id, &id2, &id1)
            }
            Lang::Repeat(body) => {
                let id1 = child(id, 1);
                self.set_last(id, vec![id1.clone()]);
                self.add_edge(id, &id1);
                self.add_edge(&id1, &id1);
                self.walk_loop(body, id, &id1, &id1)
            }
            Lang::Break => {
                self.set_last(id, vec![]);
                let target = self.breaks.last().cloned()?;
                self.add_to_last(&target, id)
            }
            Lang::Next => {
                self.set_last(id, vec![]);
                if let Some(target) = self.nexts.last().cloned() {
                    self.add_edge(id, &target);
                }
                Some(())
            }
            Lang::If { condition, consequence, alternative } => {
                let (id1, id2, id3) = (child(id, 1), child(id, 2), child(id, 3));
                self.set_last(id, vec![]);
                self.add_edge(id, &id1);
                self.walk(condition, &id1)?;

                let (true_exits, false_exits) = self.exits(&id1);
                self.add_edges_from(&true_exits, &id2);
                match alternative {
                    None => {
                        let mut last = false_exits;
                        last.push(id2.clone());
                        self.set_last(id, last);
                        self.walk(consequence, &id2)
                    }
                    Some(alternative) => {
                        self.add_edges_from(&false_exits, &id3);
                        self.set_last(id, vec![id2.clone(), id3.clone()]);
                        self.walk(consequence, &id2)?;
                        self.walk(alternative, &id3)
                    }
                }
            }
            Lang::AndOr { is_and, left, right } => {
                let (id1, id2) = (child(id, 1), child(id, 2));
                self.set_last(id, vec![]);
                self.add_edge(id, &id1);
                self.walk(left, &id1)?;

                // The right-hand side is only evaluated when the left-hand
                // side is true for `&&`, and false for `||`.
                let (left_true, left_false) = self.exits(&id1);
                let (reaches_right, left_stops) = if *is_and {
                    (left_true, left_false)
                } else {
                    (left_false, left_true)
                };
                self.add_edges_from(&reaches_right, &id2);
                self.walk(right, &id2)?;

                let (right_true, right_false) = self.exits(&id2);
                let (true_exits, false_exits) = if *is_and {
                    (right_true, [left_stops, right_false].concat())
                } else {
                    ([left_stops, right_true].concat(), right_false)
                };

                let last = [true_exits.clone(), false_exits.clone()].concat();
                self.set_last(id, last);
                self.exits.insert(id.to_string(), (true_exits, false_exits));
                Some(())
            }
            Lang::Switch { selector, arms, has_default } => {
                let id1 = child(id, 1);
                self.set_last(id, vec![]);
                self.add_edge(id, &id1);
                self.walk(selector, &id1)?;

                let mut last = vec![];
                for (i, arm) in arms.iter().enumerate() {
                    let Some(arm) = arm else {
                        continue;
                    };
                    let arm_id = child(id, i + 2);
                    self.add_edge(&id1, &arm_id);
                    self.walk(arm, &arm_id)?;
                    last.push(arm_id);
                }
                // Without a default arm, no match returns `NULL`.
                if !has_default || last.is_empty() {
                    last.push(id1);
                }
                self.set_last(id, last);
                Some(())
            }
            Lang::Call(elements) => self.walk_list(elements, id),
        }
    }

    /// Walks the body of a loop, where `break` leaves `id` and `next` goes to `next`.
    fn walk_loop(&mut self, body: &Lang, id: &str, body_id: &str, next: &str) -> Option<()> {
        self.breaks.push(id.to_string());
        self.nexts.push(next.to_string());
        let result = self.walk(body, body_id);
        self.breaks.pop();
        self.nexts.pop();
        result
    }

    fn walk_list(&mut self, elements: &[Lang], id: &str) -> Option<()> {
        let n = elements.len();
        let last = if n == 0 { vec![] } else { vec![child(id, n)] };
        self.set_last(id, last);
        if n == 0 {
            return Some(());
        }

        self.add_edge(id, &child(id, 1));
        for (i, element) in elements.iter().enumerate() {
            let i = i + 1;
            if i != 1 {
                self.add_edge(&child(id, i - 1), &child(id, i));
            }
            self.walk(element, &child(id, i))?;
        }
        Some(())
    }

    /// cyclocomp's `post_process()`: an edge leaving a node that has a `last`
    /// really leaves from the nodes in `last`, unless it goes to one of its
    /// children. Returns the number of distinct edges.
    fn resolve_edges(&self) -> usize {
        let mut resolved: HashSet<(String, String)> = HashSet::new();
        let mut seen: HashSet<(String, String)> = HashSet::new();
        let mut pending: Vec<(String, String)> = self.edges.iter().rev().cloned().collect();

        while let Some((mut from, to)) = pending.pop() {
            loop {
                // Rewriting an edge that was already seen gives the same
                // result. This also stops the rewrite chains that would never
                // finish in cyclocomp.
                if !seen.insert((from.clone(), to.clone())) {
                    break;
                }

                match self.last.get(&from) {
                    Some(last) if !last.is_empty() && !is_child(&to, &from) => {
                        for l in last[1..].iter().rev() {
                            pending.push((l.clone(), to.clone()));
                        }
                        from = last[0].clone();
                    }
                    _ => {
                        resolved.insert((from, to));
                        break;
                    }
                }
            }
        }

        resolved.len()
    }
}
