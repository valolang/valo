use valo_runtime::TypeName;

pub(super) enum Context<'a> {
    Sub,
    Function {
        return_type: TypeName,
        saw_return: &'a mut bool,
    },
}

impl<'a> Context<'a> {
    pub(super) fn reborrow(&mut self) -> Context<'_> {
        match self {
            Context::Sub => Context::Sub,
            Context::Function {
                return_type,
                saw_return,
            } => Context::Function {
                return_type: return_type.clone(),
                saw_return,
            },
        }
    }
}
