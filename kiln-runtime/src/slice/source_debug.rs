//! Source statement stops shared by real scene/attachment VMs.
use super::*;
use std::{
    collections::{BTreeSet, HashMap},
    sync::{Arc, Mutex},
};

/// Exact project-relative executable source line.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceBreakpoint {
    pub path: String,
    pub line: usize,
}
/// A suspended real gameplay VM at a statement boundary.
#[derive(Debug, Clone)]
pub struct SourceStop {
    pub owner: String,
    pub source: SourceBreakpoint,
    pub instruction: usize,
    pub stepping: bool,
}
#[derive(Default)]
struct Control {
    breaks: BTreeSet<SourceBreakpoint>,
    paused: Option<SourceStop>,
    stepping: bool,
    advanced: bool,
    skip: Option<(String, usize)>,
    muted: bool,
}
#[derive(Clone, Default)]
pub(super) struct SharedDebug(Arc<Mutex<Control>>);
impl SharedDebug {
    pub(super) fn enabled(&self) -> bool {
        let c = self.0.lock().unwrap();
        !c.muted && (!c.breaks.is_empty() || c.stepping)
    }
    pub(super) fn mute(&self, muted: bool) {
        self.0.lock().unwrap().muted = muted;
    }
    pub(super) fn abandon(&self) {
        let mut c = self.0.lock().unwrap();
        c.paused = None;
        c.stepping = false;
        c.advanced = false;
        c.skip = None;
    }
    fn probe(&self, owner: &str, ip: usize, sites: &HashMap<usize, SourceBreakpoint>) -> bool {
        let mut c = self.0.lock().unwrap();
        if c.muted {
            return false;
        }
        if c.skip
            .as_ref()
            .is_some_and(|(name, index)| name == owner && *index == ip)
        {
            c.skip = None;
            c.advanced = true;
            return false;
        }
        if let Some(source) = sites.get(&ip) {
            if c.breaks.contains(source) || (c.stepping && c.advanced) {
                let stepping = c.stepping;
                c.paused = Some(SourceStop {
                    owner: owner.into(),
                    source: source.clone(),
                    instruction: ip,
                    stepping,
                });
                c.skip = Some((owner.into(), ip));
                c.stepping = false;
                return true;
            }
        }
        c.advanced = true;
        false
    }
}
pub(super) fn sites(
    program: &Program,
    source: &str,
    origin: impl Fn(usize) -> Option<(String, usize)>,
) -> HashMap<usize, SourceBreakpoint> {
    let starts = std::iter::once(0)
        .chain(
            source
                .bytes()
                .enumerate()
                .filter(|(_, b)| *b == b'\n')
                .map(|(i, _)| i + 1),
        )
        .collect::<Vec<_>>();
    program
        .statements()
        .filter_map(|(ip, offset)| {
            let row = starts
                .partition_point(|start| *start <= offset)
                .saturating_sub(1);
            origin(row).map(|(path, line)| (ip, SourceBreakpoint { path, line }))
        })
        .collect()
}
impl SliceSession {
    /// Install an exact executable line across scene and attached imported modules.
    pub fn add_source_breakpoint(&mut self, path: &str, line: usize) -> Result<(), String> {
        if path.len() > 256 || line == 0 {
            return Err("Breakpoint needs a path up to 256 bytes and a positive line".into());
        }
        let source = SourceBreakpoint {
            path: path.into(),
            line,
        };
        if !std::iter::once(&*self)
            .chain(self.attached.iter().map(|a| &a.session))
            .any(|s| s.debug_sites.values().any(|site| site == &source))
        {
            return Err("No executable statement at that exact source line".into());
        }
        let mut c = self.source_debug.0.lock().unwrap();
        if c.breaks.len() >= 128 && !c.breaks.contains(&source) {
            return Err("At most 128 source breakpoints".into());
        }
        c.breaks.insert(source);
        Ok(())
    }
    /// Remove an exact source breakpoint. A currently suspended statement stays paused.
    pub fn remove_source_breakpoint(&mut self, path: &str, line: usize) {
        self.source_debug
            .0
            .lock()
            .unwrap()
            .breaks
            .remove(&SourceBreakpoint {
                path: path.into(),
                line,
            });
    }
    /// Configured source lines, retained independently of an execution pause.
    pub fn source_breakpoints(&self) -> Vec<SourceBreakpoint> {
        self.source_debug
            .0
            .lock()
            .unwrap()
            .breaks
            .iter()
            .cloned()
            .collect()
    }
    /// Inspect the source stop without executing any gameplay.
    pub fn source_stop(&self) -> Option<SourceStop> {
        self.source_debug.0.lock().unwrap().paused.clone()
    }
    /// Continue the pending callback/tick; the editor must also resume its clock.
    pub fn debug_continue(&mut self) {
        let mut c = self.source_debug.0.lock().unwrap();
        c.paused = None;
        c.stepping = false;
        c.advanced = false;
    }
    /// Stop at the next executed source statement, including across callback boundaries.
    pub fn debug_step_statement(&mut self) {
        let mut c = self.source_debug.0.lock().unwrap();
        c.paused = None;
        c.stepping = true;
        c.advanced = false;
    }
    pub(super) fn execute_gameplay_vm(&mut self) -> Result<(), VmError> {
        let restart = !self.program_pending;
        let result = if !self.source_debug.enabled() && restart {
            self.work_budget.execute(&mut self.vm, &self.program)
        } else {
            let debug = self.source_debug.clone();
            let owner = if self.vm.host().self_entity.is_empty() {
                "<scene>"
            } else {
                &self.vm.host().self_entity
            }
            .to_owned();
            self.work_budget
                .execute_debug(&mut self.vm, &self.program, restart, |ip, _| {
                    debug.probe(&owner, ip, &self.debug_sites)
                })
        };
        self.program_pending = matches!(result, Err(VmError::DebugSuspended));
        result
    }
}
