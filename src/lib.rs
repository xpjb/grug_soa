#[macro_export]
macro_rules! impl_load_prototype {
    // Preferred form: requires `prototype_id: usize` so prototype IDs are auto-assigned on load
    // and copied into runtime instances on spawn.
    ($ecs:ty { prototype_id: usize, $($field:ident : $ty:ty),* $(,)? }) => {
        impl $ecs {
            /// Constructs a runtime table from a prototype table.
            ///
            /// This seeds any `Overlay<T>` fields with prototype `dense_data`, so runtime reads
            /// can fall back to prototypes without copying per-instance data up-front.
            pub fn new_from_prototypes(prototypes: &$ecs) -> Self
            where
                $(
                    $ty: ::core::clone::Clone + ::core::default::Default + ::serde::de::DeserializeOwned,
                )*
            {
                let mut out = <$ecs as ::core::default::Default>::default();

                $(
                    <_ as $crate::Storage<$ty>>::init_from_prototypes(
                        &mut out.$field,
                        &prototypes.$field,
                    );
                )*

                out
            }

            /// Copies (clones) one "entity" worth of registered component fields from `prototype`
            /// into `self`, chosen by `prototype_index`.
            ///
            /// This is intended for runtime spawning, where no deserialization happens.
            pub fn spawn_entity(&mut self, prototype: &$ecs, prototype_index: usize)
            where
                $(
                    // The spawned table may store either a dense Vec<T> or a sparse Overlay<T>.
                    $ty: ::core::clone::Clone + ::core::default::Default + ::serde::de::DeserializeOwned,
                )*
            {
                // Always copy prototype_id to instances (caller can store prototype_id as a normal field).
                self.prototype_id.push(prototype.prototype_id[prototype_index]);
                $(
                    <_ as $crate::Storage<$ty>>::push_from_prototype(
                        &mut self.$field,
                        &prototype.$field,
                        prototype_index,
                    );
                )*
            }

            /// Removes an entity by index using `Vec::swap_remove` for every registered field.
            ///
            /// This is an O(1) removal but does **not** preserve ordering (the last entity is moved
            /// into `index`).
            pub fn swap_remove(&mut self, index: usize) {
                self.prototype_id.swap_remove(index);
                $(
                    <_ as $crate::Storage<$ty>>::swap_remove(&mut self.$field, index);
                )*
            }

            /// Loads a prototype from a JSON object into the ECS.
            pub fn load_prototype(&mut self, prototype: ::serde_json::Value) {
                let obj = prototype
                    .as_object()
                    .expect("prototype must be a JSON object");
                let null = ::serde_json::Value::Null;

                $(
                    if let Some(v) = obj.get(::core::stringify!($field)) {
                        <_ as $crate::Storage<$ty>>::push_json(&mut self.$field, v);
                    } else {
                        // Missing field => default prototype value, regardless of storage backend.
                        <_ as $crate::Storage<$ty>>::push_json(&mut self.$field, &null);
                    }
                )*

                // Auto-assign prototype_id if the JSON didn't include it (or if it did; we ignore it).
                let next_id = self.prototype_id.len();
                self.prototype_id.push(next_id);
            }
        }
    };

    // Explicit error for old macro call sites that don't declare prototype_id.
    ($ecs:ty { $($field:ident : $ty:ty),* $(,)? }) => {
        compile_error!(
            "impl_load_prototype!(...) now requires `prototype_id: usize` as the first field in the macro invocation, so prototype IDs can be auto-assigned and copied on spawn."
        );
    };
}

use std::collections::HashMap;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// The secret sauce: A trait that masks the difference between Vec and Overlay
pub trait Storage<T> {
    /// Initialize a runtime table field from the prototypes table.
    ///
    /// - For dense `Vec<T>`: no-op (runtime instances start empty).
    /// - For `Overlay<T>`: clones prototype `dense_data` so runtime lookups can fall back.
    fn init_from_prototypes(&mut self, prototypes: &Self);
    fn push_json(&mut self, json: &Value);
    fn push_from_prototype(&mut self, source: &Self, proto_idx: usize);
    fn swap_remove(&mut self, index: usize);
}

// Implement for standard Vec (Dense storage)
impl<T> Storage<T> for Vec<T> 
where T: Clone + Default + DeserializeOwned 
{
    fn init_from_prototypes(&mut self, _prototypes: &Self) {
        // Runtime instance storage starts empty for dense Vec.
    }

    fn push_json(&mut self, json: &Value) {
        let val = serde_json::from_value::<T>(json.clone()).unwrap_or_default();
        self.push(val);
    }

    fn push_from_prototype(&mut self, source: &Self, proto_idx: usize) {
        self.push(source[proto_idx].clone());
    }

    fn swap_remove(&mut self, index: usize) {
        self.swap_remove(index);
    }
}

// Implement for Overlay (Sparse/COW storage)
impl<T> Storage<T> for Overlay<T> 
where T: Clone + Default + DeserializeOwned 
{
    fn init_from_prototypes(&mut self, prototypes: &Self) {
        // Copy prototype/template data; clear per-instance overrides.
        self.dense_data = prototypes.dense_data.clone();
        self.sparse_data.clear();
        self.presence.clear();
        self.instance_len = 0;
    }

    fn push_json(&mut self, json: &Value) {
        // For an overlay, loading a prototype appends to `dense_data` (the templates).
        let val = serde_json::from_value::<T>(json.clone()).unwrap_or_default();
        self.dense_data.push(val);
    }

    fn push_from_prototype(&mut self, _source: &Self, _proto_idx: usize) {
        // When spawning a LIVE entity, an Overlay doesn't copy prototype data yet.
        // It just extends the instance bitmask (copy-on-write on first mutation).
        self.push_instance();
    }

    fn swap_remove(&mut self, index: usize) {
        self.swap_remove_instance(index);
    }
}

// Honestly its kind of just fat so we can use it in both tables to simplify our shit
// Can be used for - sparse data , defaults, copy on write data
#[derive(Clone)]
pub struct Overlay<T> {
    /// Prototype/template data (indexed by `prototype_id`).
    pub dense_data: Vec<T>,

    /// Per-instance overrides (keyed by `instance_id`).
    pub sparse_data: HashMap<usize, T>,

    /// Bitmask for which instances have overrides in `sparse_data`.
    /// One bit per instance: 1 => present in `sparse_data`, 0 => use prototype fallback.
    pub presence: Vec<u64>,

    /// Logical number of instances being tracked by this overlay.
    instance_len: usize,
}

impl<T> Default for Overlay<T> {
    fn default() -> Self {
        Self {
            dense_data: Vec::new(),
            sparse_data: HashMap::new(),
            presence: Vec::new(),
            instance_len: 0,
        }
    }
}

impl<T> Overlay<T>
where
    T: Clone,
{
    #[inline]
    fn word_bit(instance_id: usize) -> (usize, u64) {
        let word = instance_id >> 6;
        let bit = instance_id & 63;
        (word, 1u64 << bit)
    }

    #[inline]
    fn ensure_presence_capacity(&mut self, instance_id: usize) {
        let (word, _) = Self::word_bit(instance_id);
        if self.presence.len() <= word {
            self.presence.resize(word + 1, 0);
        }
    }

    /// Number of spawned instances represented by this overlay.
    pub fn instances_len(&self) -> usize {
        self.instance_len
    }

    /// Number of loaded prototypes/templates represented by this overlay.
    pub fn prototypes_len(&self) -> usize {
        self.dense_data.len()
    }

    /// Adds a new instance slot (no override set).
    pub fn push_instance(&mut self) {
        let id = self.instance_len;
        self.instance_len += 1;
        self.ensure_presence_capacity(id);
        // bit defaults to 0 => no override
    }

    /// Returns true if this instance has an override.
    pub fn has_override(&self, instance_id: usize) -> bool {
        if instance_id >= self.instance_len {
            return false;
        }
        let (word, mask) = Self::word_bit(instance_id);
        (self.presence.get(word).copied().unwrap_or(0) & mask) != 0
    }

    /// Clears an override for `instance_id`, if present.
    pub fn clear_override(&mut self, instance_id: usize) {
        if instance_id >= self.instance_len {
            return;
        }
        let (word, mask) = Self::word_bit(instance_id);
        if let Some(w) = self.presence.get_mut(word) {
            *w &= !mask;
        }
        self.sparse_data.remove(&instance_id);
    }

    /// Read with fallback to prototype/template data.
    ///
    /// Requires `prototype_id` to be known by the caller (stored as a normal field on the SoA).
    pub fn get(&self, instance_id: usize, prototype_id: usize) -> &T { // maybe we can grab prototype id with a macro
        if self.has_override(instance_id) {
            return self
                .sparse_data
                .get(&instance_id)
                .expect("Overlay presence bit set but sparse_data missing entry");
        }
        &self.dense_data[prototype_id]
    }

    /// Write access with copy-on-write from the prototype/template.
    pub fn get_mut(&mut self, instance_id: usize, prototype_id: usize) -> &mut T {
        if instance_id >= self.instance_len {
            panic!("Overlay get_mut out of bounds: {instance_id} >= {}", self.instance_len);
        }

        if !self.has_override(instance_id) {
            let base = self.dense_data[prototype_id].clone();
            self.sparse_data.insert(instance_id, base);
            let (word, mask) = Self::word_bit(instance_id);
            self.ensure_presence_capacity(instance_id);
            self.presence[word] |= mask;
        }

        self.sparse_data
            .get_mut(&instance_id)
            .expect("Overlay write: sparse_data missing entry after insert")
    }

    /// Sets an override value for `instance_id` (marks presence bit).
    pub fn set(&mut self, instance_id: usize, value: T) {
        if instance_id >= self.instance_len {
            panic!("Overlay set out of bounds: {instance_id} >= {}", self.instance_len);
        }
        self.sparse_data.insert(instance_id, value);
        let (word, mask) = Self::word_bit(instance_id);
        self.ensure_presence_capacity(instance_id);
        self.presence[word] |= mask;
    }

    /// Swap-remove an instance slot, keeping O(1) semantics consistent with `Vec::swap_remove`.
    ///
    /// If the last instance had an override, it is moved into `index`.
    pub fn swap_remove_instance(&mut self, index: usize) {
        if index >= self.instance_len {
            panic!(
                "Overlay swap_remove out of bounds: {index} >= {}",
                self.instance_len
            );
        }

        let last = self.instance_len - 1;

        // Remove index override (if any).
        self.clear_override(index);

        if index != last {
            let last_has = self.has_override(last);

            if last_has {
                if let Some(v) = self.sparse_data.remove(&last) {
                    self.sparse_data.insert(index, v);
                } else {
                    panic!("Overlay presence bit set for last but sparse_data missing entry");
                }
            }

            // Copy last presence bit to index.
            let (iw, im) = Self::word_bit(index);
            let (lw, lm) = Self::word_bit(last);
            self.ensure_presence_capacity(index);
            self.ensure_presence_capacity(last);

            if last_has {
                self.presence[iw] |= im;
            } else {
                self.presence[iw] &= !im;
            }

            // Clear last bit.
            self.presence[lw] &= !lm;
        } else {
            // Clear last bit (already cleared by clear_override(index)).
            let (lw, lm) = Self::word_bit(last);
            if let Some(w) = self.presence.get_mut(lw) {
                *w &= !lm;
            }
        }

        self.instance_len -= 1;
    }
}