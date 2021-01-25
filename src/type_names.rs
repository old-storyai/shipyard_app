use std::{
    any::{type_name, TypeId},
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Default)]
pub(crate) struct TypeNames(Rc<RefCell<HashMap<TypeId, &'static str>>>);

impl TypeNames {
    pub(crate) fn tracked_type_id_of<T: 'static>(&self) -> TypeId {
        let type_id = TypeId::of::<T>();
        self.0
            .borrow_mut()
            .entry(type_id)
            .or_insert_with(type_name::<T>);
        type_id
    }

    pub fn lookup_name(&self, type_id: &TypeId) -> Option<&'static str> {
        self.0.borrow().get(type_id).copied()
    }
}
