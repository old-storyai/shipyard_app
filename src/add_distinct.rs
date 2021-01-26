use shipyard::*;

pub trait AddDistinct: AddComponent {
    /// Assign this component if the value is distinct from that which is already in the storage.
    ///
    ///  * If the component doesn't exist already, it will be inserted.
    ///  * If the component already exists, and is not equal the component will be modified to the new value.
    ///  * If the component already exists, and is equal no mutation will occur (`update_pack` will remain clean).
    fn add_distinct(&mut self, entity: EntityId, component: Self::Component) -> bool;
}

impl<T: 'static + PartialEq> AddDistinct for ViewMut<'_, T> {
    fn add_distinct(&mut self, entity: EntityId, component: Self::Component) -> bool {
        if let Ok(has_value) = (&*self).get(entity) {
            if &component == has_value {
                return false;
            }
        }

        self.add_component_unchecked(entity, component);
        return true;
    }
}

macro_rules! impl_add_distinct {
    ($(($type: ident, $index: tt))+) => {
        impl<$($type: 'static + PartialEq),+> AddDistinct for ($(&mut ViewMut<'_, $type>,)+) {
            fn add_distinct(&mut self, entity: EntityId, component: Self::Component) -> bool {
                if let Ok(has_value) = ($(&*self.$index,)+).get(entity) {
                    if ($(&component.$index,)+) == has_value {
                        return false;
                    }
                }

                $(
                    self.$index.add_component_unchecked(entity, component.$index);
                )+
                return true;
            }
        }
    }
}

macro_rules! add_distinct {
    ($(($type: ident, $index: tt))+; ($type1: ident, $index1: tt) $(($queue_type: ident, $queue_index: tt))*) => {
        impl_add_distinct![$(($type, $index))*];
        add_distinct![$(($type, $index))* ($type1, $index1); $(($queue_type, $queue_index))*];
    };
    ($(($type: ident, $index: tt))+;) => {
        impl_add_distinct![$(($type, $index))*];
    }
}

add_distinct![(A, 0); (B, 1) (C, 2) (D, 3)];
