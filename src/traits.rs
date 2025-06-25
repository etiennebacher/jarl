use air_r_syntax::{RArgument, RArgumentList};
use biome_rowan::AstSeparatedList;

pub trait ArgumentListExt {
    fn get_arg_by_name(&self, name: &str) -> Option<RArgument>;
    fn get_arg_by_position(&self, pos: usize) -> Option<RArgument>;
    fn get_arg_by_name_then_position(&self, name: &str, pos: usize) -> Option<RArgument>;
}

impl ArgumentListExt for RArgumentList {
    fn get_arg_by_name(&self, name: &str) -> Option<RArgument> {
        self.into_iter()
            .find(|x| {
                let name_clause = x.clone().unwrap().name_clause();
                if let Some(name_clause) = name_clause {
                    if let Ok(name_clause) = name_clause.name() {
                        name_clause.to_string().trim() == name
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
            .map(|x| x.unwrap())
    }

    fn get_arg_by_position(&self, pos: usize) -> Option<RArgument> {
        self.iter().nth(pos).map(|x| x.unwrap())
    }

    fn get_arg_by_name_then_position(&self, name: &str, pos: usize) -> Option<RArgument> {
        if let Some(by_name) = self.get_arg_by_name(name) {
            Some(by_name)
        } else {
            self.get_arg_by_position(pos)
        }
    }
}
