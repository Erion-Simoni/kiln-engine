//! Declared component access for parallel ECS scheduling.
//!
//! Systems report which component types they read and write. Conflicting
//! access (write/write or read/write on the same type) means two systems
//! must not run in the same parallel stage.

use std::any::TypeId;
use std::collections::HashSet;

/// Whether a system reads or writes a component type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AccessKind {
    Read,
    Write,
}

/// One component access declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ComponentAccess {
    pub type_id: TypeId,
    pub kind: AccessKind,
}

impl ComponentAccess {
    pub fn read<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            kind: AccessKind::Read,
        }
    }

    pub fn write<T: 'static>() -> Self {
        Self {
            type_id: TypeId::of::<T>(),
            kind: AccessKind::Write,
        }
    }
}

/// Set of component reads and writes for one system.
#[derive(Debug, Clone, Default)]
pub struct AccessSet {
    reads: HashSet<TypeId>,
    writes: HashSet<TypeId>,
}

impl AccessSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn read<T: 'static>(mut self) -> Self {
        self.reads.insert(TypeId::of::<T>());
        self
    }

    pub fn write<T: 'static>(mut self) -> Self {
        self.writes.insert(TypeId::of::<T>());
        self
    }

    pub fn insert(&mut self, access: ComponentAccess) {
        match access.kind {
            AccessKind::Read => {
                self.reads.insert(access.type_id);
            }
            AccessKind::Write => {
                self.writes.insert(access.type_id);
            }
        }
    }

    pub fn from_accesses(accesses: &[ComponentAccess]) -> Self {
        let mut set = Self::new();
        for a in accesses {
            set.insert(*a);
        }
        set
    }

    pub fn reads(&self) -> &HashSet<TypeId> {
        &self.reads
    }

    pub fn writes(&self) -> &HashSet<TypeId> {
        &self.writes
    }

    /// True if this set conflicts with another (cannot share a parallel stage).
    ///
    /// Conflict rules:
    /// - write/write on the same type
    /// - read/write on the same type (either direction)
    pub fn conflicts_with(&self, other: &AccessSet) -> bool {
        for w in &self.writes {
            if other.writes.contains(w) || other.reads.contains(w) {
                return true;
            }
        }
        for w in &other.writes {
            if self.reads.contains(w) {
                return true;
            }
        }
        false
    }

    /// True if every access is read-only (safe to share `&World` across threads).
    pub fn is_read_only(&self) -> bool {
        self.writes.is_empty()
    }
}

/// Returns true if any pair in `sets` conflicts.
pub fn any_conflicts(sets: &[AccessSet]) -> bool {
    for i in 0..sets.len() {
        for j in (i + 1)..sets.len() {
            if sets[i].conflicts_with(&sets[j]) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Position;
    struct Velocity;
    struct Health;

    #[test]
    fn disjoint_writes_do_not_conflict() {
        let a = AccessSet::new().write::<Position>();
        let b = AccessSet::new().write::<Velocity>();
        assert!(!a.conflicts_with(&b));
    }

    #[test]
    fn same_write_conflicts() {
        let a = AccessSet::new().write::<Position>();
        let b = AccessSet::new().write::<Position>();
        assert!(a.conflicts_with(&b));
    }

    #[test]
    fn read_write_same_type_conflicts() {
        let a = AccessSet::new().read::<Position>();
        let b = AccessSet::new().write::<Position>();
        assert!(a.conflicts_with(&b));
        assert!(b.conflicts_with(&a));
    }

    #[test]
    fn reads_same_type_do_not_conflict() {
        let a = AccessSet::new().read::<Position>().read::<Velocity>();
        let b = AccessSet::new().read::<Position>().read::<Health>();
        assert!(!a.conflicts_with(&b));
    }

    #[test]
    fn write_and_unrelated_read_ok() {
        let a = AccessSet::new().write::<Position>();
        let b = AccessSet::new().read::<Velocity>();
        assert!(!a.conflicts_with(&b));
    }

    #[test]
    fn any_conflicts_detects_pair() {
        let sets = vec![
            AccessSet::new().read::<Health>(),
            AccessSet::new().write::<Position>(),
            AccessSet::new().write::<Position>(),
        ];
        assert!(any_conflicts(&sets));
    }

    #[test]
    fn any_conflicts_clean_stage() {
        let sets = vec![
            AccessSet::new().write::<Position>(),
            AccessSet::new().write::<Velocity>(),
            AccessSet::new().read::<Health>(),
        ];
        assert!(!any_conflicts(&sets));
    }

    #[test]
    fn read_only_flag() {
        assert!(AccessSet::new().read::<Position>().is_read_only());
        assert!(!AccessSet::new().write::<Position>().is_read_only());
    }

    #[test]
    fn from_accesses_builder() {
        let set = AccessSet::from_accesses(&[
            ComponentAccess::read::<Position>(),
            ComponentAccess::write::<Velocity>(),
        ]);
        assert!(set.reads().contains(&TypeId::of::<Position>()));
        assert!(set.writes().contains(&TypeId::of::<Velocity>()));
    }
}
