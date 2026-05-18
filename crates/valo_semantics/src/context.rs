use valo_runtime::TypeName;

pub(super) enum Context<'a> {
    Sub,
    Function {
        return_type: TypeName,
        saw_return: &'a mut bool,
    },
    MethodSub {
        class_name: String,
    },
    MethodFunction {
        class_name: String,
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
            Context::MethodSub { class_name } => Context::MethodSub {
                class_name: class_name.clone(),
            },
            Context::MethodFunction {
                class_name,
                return_type,
                saw_return,
            } => Context::MethodFunction {
                class_name: class_name.clone(),
                return_type: return_type.clone(),
                saw_return,
            },
        }
    }

    pub(super) fn current_class(&self) -> Option<&str> {
        match self {
            Context::MethodSub { class_name } | Context::MethodFunction { class_name, .. } => {
                Some(class_name)
            }
            _ => None,
        }
    }
}
