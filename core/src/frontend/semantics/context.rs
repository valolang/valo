use crate::runtime::TypeName;

pub(super) enum Context<'a> {
    Sub,
    Function {
        return_type: TypeName,
        return_slot: Option<String>,
        is_iterator: bool,
        saw_return: &'a mut bool,
        saw_yield: &'a mut bool,
    },
    MethodSub {
        class_name: String,
    },
    MethodFunction {
        class_name: String,
        return_type: TypeName,
        return_slot: Option<String>,
        is_iterator: bool,
        saw_return: &'a mut bool,
        saw_yield: &'a mut bool,
    },
    PropertyGet {
        class_name: String,
        return_type: TypeName,
        return_slot: Option<String>,
        is_iterator: bool,
        saw_return: &'a mut bool,
        saw_yield: &'a mut bool,
    },
    PropertyLetSet {
        class_name: String,
    },
}

impl<'a> Context<'a> {
    pub(super) fn reborrow(&mut self) -> Context<'_> {
        match self {
            Context::Sub => Context::Sub,
            Context::Function {
                return_type,
                return_slot,
                is_iterator,
                saw_return,
                saw_yield,
            } => Context::Function {
                return_type: return_type.clone(),
                return_slot: return_slot.clone(),
                is_iterator: *is_iterator,
                saw_return,
                saw_yield,
            },
            Context::MethodSub { class_name } => Context::MethodSub {
                class_name: class_name.clone(),
            },
            Context::MethodFunction {
                class_name,
                return_type,
                return_slot,
                is_iterator,
                saw_return,
                saw_yield,
            } => Context::MethodFunction {
                class_name: class_name.clone(),
                return_type: return_type.clone(),
                return_slot: return_slot.clone(),
                is_iterator: *is_iterator,
                saw_return,
                saw_yield,
            },
            Context::PropertyGet {
                class_name,
                return_type,
                return_slot,
                is_iterator,
                saw_return,
                saw_yield,
            } => Context::PropertyGet {
                class_name: class_name.clone(),
                return_type: return_type.clone(),
                return_slot: return_slot.clone(),
                is_iterator: *is_iterator,
                saw_return,
                saw_yield,
            },
            Context::PropertyLetSet { class_name } => Context::PropertyLetSet {
                class_name: class_name.clone(),
            },
        }
    }

    pub(super) fn current_class(&self) -> Option<&str> {
        match self {
            Context::MethodSub { class_name }
            | Context::MethodFunction { class_name, .. }
            | Context::PropertyGet { class_name, .. }
            | Context::PropertyLetSet { class_name } => Some(class_name),
            _ => None,
        }
    }
}
