use crate::runtime::TypeName;

pub(super) enum Context<'a> {
    Sub {
        is_async: bool,
    },
    Function {
        return_type: TypeName,
        return_slot: Option<String>,
        is_iterator: bool,
        is_async: bool,
        saw_return: &'a mut bool,
        saw_yield: &'a mut bool,
    },
    MethodSub {
        class_name: String,
        is_async: bool,
    },
    MethodFunction {
        class_name: String,
        return_type: TypeName,
        return_slot: Option<String>,
        is_iterator: bool,
        is_async: bool,
        saw_return: &'a mut bool,
        saw_yield: &'a mut bool,
    },
    PropertyGet {
        class_name: String,
        return_type: TypeName,
        return_slot: Option<String>,
        is_iterator: bool,
        is_async: bool,
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
            Context::Sub { is_async } => Context::Sub {
                is_async: *is_async,
            },
            Context::Function {
                return_type,
                return_slot,
                is_iterator,
                is_async,
                saw_return,
                saw_yield,
            } => Context::Function {
                return_type: return_type.clone(),
                return_slot: return_slot.clone(),
                is_iterator: *is_iterator,
                is_async: *is_async,
                saw_return,
                saw_yield,
            },
            Context::MethodSub {
                class_name,
                is_async,
            } => Context::MethodSub {
                class_name: class_name.clone(),
                is_async: *is_async,
            },
            Context::MethodFunction {
                class_name,
                return_type,
                return_slot,
                is_iterator,
                is_async,
                saw_return,
                saw_yield,
            } => Context::MethodFunction {
                class_name: class_name.clone(),
                return_type: return_type.clone(),
                return_slot: return_slot.clone(),
                is_iterator: *is_iterator,
                is_async: *is_async,
                saw_return,
                saw_yield,
            },
            Context::PropertyGet {
                class_name,
                return_type,
                return_slot,
                is_iterator,
                is_async,
                saw_return,
                saw_yield,
            } => Context::PropertyGet {
                class_name: class_name.clone(),
                return_type: return_type.clone(),
                return_slot: return_slot.clone(),
                is_iterator: *is_iterator,
                is_async: *is_async,
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
            Context::MethodSub { class_name, .. }
            | Context::MethodFunction { class_name, .. }
            | Context::PropertyGet { class_name, .. }
            | Context::PropertyLetSet { class_name } => Some(class_name),
            _ => None,
        }
    }

    pub(super) fn allows_await(&self) -> bool {
        match self {
            Context::Sub { is_async }
            | Context::Function { is_async, .. }
            | Context::MethodSub { is_async, .. }
            | Context::MethodFunction { is_async, .. }
            | Context::PropertyGet { is_async, .. } => *is_async,
            Context::PropertyLetSet { .. } => false,
        }
    }
}
