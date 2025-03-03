use crate::message::*;
use crate::{BindingId, SemanticModel};

pub fn check_unused_variables(model: &SemanticModel) -> Vec<Diagnostic> {
    let scopes = &model.data.scopes;
    let mut diagnostics = vec![];

    for scope in scopes.iter() {
        // println!("scope: {:#?}", scope);

        let bindings = &scope.bindings_by_name;

        let bindings_read_in_scope = &scope
            .read_references
            .iter()
            .map(|reference| reference.binding_id())
            .collect::<Vec<BindingId>>();

        let bindings_written_in_scope = &scope
            .write_references
            .iter()
            .map(|reference| reference.binding_id())
            .collect::<Vec<BindingId>>();

        for binding in bindings.iter() {
            let binding_was_written_here = bindings_written_in_scope.contains(binding.1);
            let binding_was_read_here = bindings_read_in_scope.contains(binding.1);

            if binding_was_written_here && !binding_was_read_here {
                // println!("UNUSED BINDING: {:?}", binding.0);
                // diagnostics.push(Diagnostic::UnusedObjs {
                //     // filename: file.into(),
                //     // location: Location { row, column },
                //     varname: binding.0.to_string(),
                // })
            }

            if !binding_was_written_here && binding_was_read_here {
                // println!("UNUSED BINDING: {:?}", binding.0);
                // diagnostics.push(Diagnostic::UndefinedObjs {
                //     // filename: file.into(),
                //     // location: Location { row, column },
                //     varname: binding.0.to_string(),
                // })
            }
        }
    }

    diagnostics
}
