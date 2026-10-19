//! ECS system schedule with declared access and parallel stages.
//!
//! Systems declare an [`AccessSet`]. The schedule packs non-conflicting
//! systems into stages. World systems always run with exclusive `&mut World`
//! (hecs requirement). Independent non-world jobs in a stage can run on a
//! [`TaskPool`] when the `threads` feature is enabled.

use crate::jobs::TaskPool;
use hecs::World;
use kiln_core::{any_conflicts, AccessSet};

/// A scheduled system: declared access plus a runnable body.
pub struct SystemEntry {
    pub name: String,
    pub access: AccessSet,
    run: Box<dyn FnMut(&mut World) + Send>,
}

impl SystemEntry {
    pub fn new(
        name: impl Into<String>,
        access: AccessSet,
        run: impl FnMut(&mut World) + Send + 'static,
    ) -> Self {
        Self {
            name: name.into(),
            access,
            run: Box::new(run),
        }
    }
}

/// Ordered stages of system indices. Systems in one stage do not conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePlan {
    pub stages: Vec<Vec<usize>>,
}

/// Build a greedy stage plan: each system joins the earliest stage that
/// has no access conflict with systems already in that stage.
pub fn build_stages(accesses: &[AccessSet]) -> SchedulePlan {
    let mut stages: Vec<Vec<usize>> = Vec::new();
    let mut stage_access: Vec<Vec<AccessSet>> = Vec::new();

    for (idx, access) in accesses.iter().enumerate() {
        let mut placed = false;
        for (s, existing) in stage_access.iter_mut().enumerate() {
            let mut trial = existing.clone();
            trial.push(access.clone());
            if !any_conflicts(&trial) {
                stages[s].push(idx);
                existing.push(access.clone());
                placed = true;
                break;
            }
        }
        if !placed {
            stages.push(vec![idx]);
            stage_access.push(vec![access.clone()]);
        }
    }

    SchedulePlan { stages }
}

/// Parallel-aware ECS schedule.
pub struct Schedule {
    systems: Vec<SystemEntry>,
    plan: SchedulePlan,
    dirty: bool,
}

impl Schedule {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
            plan: SchedulePlan { stages: Vec::new() },
            dirty: false,
        }
    }

    pub fn add(&mut self, entry: SystemEntry) {
        self.systems.push(entry);
        self.dirty = true;
    }

    pub fn system_count(&self) -> usize {
        self.systems.len()
    }

    pub fn plan(&mut self) -> &SchedulePlan {
        self.rebuild_if_dirty();
        &self.plan
    }

    fn rebuild_if_dirty(&mut self) {
        if self.dirty {
            let accesses: Vec<_> = self.systems.iter().map(|s| s.access.clone()).collect();
            self.plan = build_stages(&accesses);
            self.dirty = false;
        }
    }

    /// Run all systems in stage order. Systems within a stage are non-conflicting
    /// by construction; each still receives exclusive `&mut World` sequentially
    /// because hecs queries require it for writes.
    pub fn run(&mut self, world: &mut World) {
        self.rebuild_if_dirty();
        let plan = self.plan.clone();
        for stage in &plan.stages {
            for &i in stage {
                (self.systems[i].run)(world);
            }
        }
    }

    /// Run independent closures that correspond 1:1 with systems in `stage_index`.
    ///
    /// Used when system work has been extracted from the world (e.g. parallel
    /// transform updates). Validates that the stage is conflict-free, then
    /// executes jobs on `pool` (parallel when `threads` is on).
    pub fn run_stage_jobs(
        &mut self,
        stage_index: usize,
        pool: &TaskPool,
        jobs: &mut [Box<dyn FnMut() + Send>],
    ) {
        self.rebuild_if_dirty();
        let stage = self
            .plan
            .stages
            .get(stage_index)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            stage.len(),
            jobs.len(),
            "job count must match systems in stage"
        );

        let accesses: Vec<_> = stage
            .iter()
            .map(|&i| self.systems[i].access.clone())
            .collect();
        assert!(
            !any_conflicts(&accesses),
            "refusing to run conflicting systems in parallel"
        );

        #[cfg(all(feature = "threads", not(target_arch = "wasm32")))]
        if pool.is_parallel() && jobs.len() > 1 {
            // Scoped threads need non-overlapping `&mut` to each boxed job.
            let mut slots: Vec<Option<&mut (dyn FnMut() + Send)>> =
                jobs.iter_mut().map(|j| Some(j.as_mut())).collect();
            std::thread::scope(|s| {
                for slot in slots.iter_mut() {
                    let job = slot.take().expect("job slot");
                    s.spawn(job);
                }
            });
            return;
        }

        #[cfg(not(all(feature = "threads", not(target_arch = "wasm32"))))]
        let _ = pool;

        for job in jobs.iter_mut() {
            job();
        }
    }
}

impl Default for Schedule {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kiln_core::AccessSet;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct Pos;
    struct Vel;
    struct Hp;

    #[test]
    fn conflicting_systems_get_separate_stages() {
        let accesses = vec![
            AccessSet::new().write::<Pos>(),
            AccessSet::new().write::<Pos>(),
            AccessSet::new().write::<Vel>(),
        ];
        let plan = build_stages(&accesses);
        assert!(plan.stages.len() >= 2);
        let together = plan.stages.iter().any(|s| s.contains(&0) && s.contains(&1));
        assert!(!together, "conflicting Pos writers must not share a stage");
    }

    #[test]
    fn nonconflicting_share_stage() {
        let accesses = vec![
            AccessSet::new().write::<Pos>(),
            AccessSet::new().write::<Vel>(),
            AccessSet::new().read::<Hp>(),
        ];
        let plan = build_stages(&accesses);
        assert_eq!(plan.stages.len(), 1);
        assert_eq!(plan.stages[0], vec![0, 1, 2]);
    }

    #[test]
    fn schedule_runs_all_systems() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut schedule = Schedule::new();
        for name in ["a", "b", "c"] {
            let c = Arc::clone(&counter);
            schedule.add(SystemEntry::new(
                name,
                AccessSet::new().write::<Pos>(),
                move |_world| {
                    c.fetch_add(1, Ordering::SeqCst);
                },
            ));
        }
        let mut world = World::new();
        schedule.run(&mut world);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[test]
    fn plan_exposes_stages() {
        let mut schedule = Schedule::new();
        schedule.add(SystemEntry::new(
            "pos",
            AccessSet::new().write::<Pos>(),
            |_| {},
        ));
        schedule.add(SystemEntry::new(
            "vel",
            AccessSet::new().write::<Vel>(),
            |_| {},
        ));
        let plan = schedule.plan().clone();
        assert_eq!(plan.stages.len(), 1);
    }

    #[test]
    fn run_stage_jobs_executes() {
        let counter = Arc::new(AtomicU32::new(0));
        let mut schedule = Schedule::new();
        schedule.add(SystemEntry::new(
            "pos",
            AccessSet::new().write::<Pos>(),
            |_| {},
        ));
        schedule.add(SystemEntry::new(
            "vel",
            AccessSet::new().write::<Vel>(),
            |_| {},
        ));
        let pool = TaskPool::new(2);
        let c0 = Arc::clone(&counter);
        let c1 = Arc::clone(&counter);
        let mut jobs: Vec<Box<dyn FnMut() + Send>> = vec![
            Box::new(move || {
                c0.fetch_add(1, Ordering::SeqCst);
            }),
            Box::new(move || {
                c1.fetch_add(10, Ordering::SeqCst);
            }),
        ];
        schedule.run_stage_jobs(0, &pool, &mut jobs);
        assert_eq!(counter.load(Ordering::SeqCst), 11);
    }

    #[test]
    fn conflict_helper_matches_access_sets() {
        let ok = vec![
            AccessSet::new().write::<Pos>(),
            AccessSet::new().write::<Vel>(),
        ];
        assert!(!any_conflicts(&ok));
        let bad = vec![
            AccessSet::new().write::<Pos>(),
            AccessSet::new().write::<Pos>(),
        ];
        assert!(any_conflicts(&bad));
    }
}
