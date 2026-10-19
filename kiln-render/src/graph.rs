//! Frame / render graph: declared passes, resource lifetimes, transient aliasing.

use std::collections::HashMap;

/// Stable handle for a graph resource (texture or buffer descriptor).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u32);

/// Stable handle for a pass node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PassId(pub u32);

/// Declared texture / buffer shape (not yet a GPU object).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResourceKind {
    /// 2D texture used as color / depth attachment or sampled input.
    Texture {
        width: u32,
        height: u32,
        format: TextureFormatKey,
    },
    /// Opaque buffer placeholder (size bytes).
    Buffer { size: u64 },
}

/// Subset of wgpu formats we care about for aliasing equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextureFormatKey {
    Rgba8Unorm,
    Rgba16Float,
    Depth24Plus,
    Surface,
}

impl TextureFormatKey {
    /// Map to wgpu when creating real textures (Surface is caller-provided).
    pub fn to_wgpu(self) -> Option<wgpu::TextureFormat> {
        match self {
            TextureFormatKey::Rgba8Unorm => Some(wgpu::TextureFormat::Rgba8Unorm),
            TextureFormatKey::Rgba16Float => Some(wgpu::TextureFormat::Rgba16Float),
            TextureFormatKey::Depth24Plus => Some(wgpu::TextureFormat::Depth24Plus),
            TextureFormatKey::Surface => None,
        }
    }
}

/// Named resource declaration.
#[derive(Debug, Clone)]
pub struct ResourceDesc {
    pub name: String,
    pub kind: ResourceKind,
    /// When true, may share a physical slot with non-overlapping peers.
    pub transient: bool,
}

/// How a pass touches a resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceUsage {
    Read,
    Write,
}

/// One edge from a pass to a resource.
#[derive(Debug, Clone, Copy)]
pub struct ResourceRef {
    pub id: ResourceId,
    pub usage: ResourceUsage,
}

/// Declared pass (name + resource edges). Execute callbacks live outside compile.
#[derive(Debug, Clone)]
pub struct PassDecl {
    pub name: String,
    pub resources: Vec<ResourceRef>,
}

/// Builder for a frame graph.
#[derive(Debug, Default)]
pub struct RenderGraph {
    resources: Vec<ResourceDesc>,
    passes: Vec<PassDecl>,
    name_to_resource: HashMap<String, ResourceId>,
}

impl RenderGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a resource; returns its id. Panics on duplicate name.
    pub fn add_resource(&mut self, desc: ResourceDesc) -> ResourceId {
        assert!(
            !self.name_to_resource.contains_key(&desc.name),
            "duplicate resource {}",
            desc.name
        );
        let id = ResourceId(self.resources.len() as u32);
        self.name_to_resource.insert(desc.name.clone(), id);
        self.resources.push(desc);
        id
    }

    pub fn resource_id(&self, name: &str) -> Option<ResourceId> {
        self.name_to_resource.get(name).copied()
    }

    pub fn add_pass(&mut self, pass: PassDecl) -> PassId {
        let id = PassId(self.passes.len() as u32);
        self.passes.push(pass);
        id
    }

    pub fn passes(&self) -> &[PassDecl] {
        &self.passes
    }

    pub fn resources(&self) -> &[ResourceDesc] {
        &self.resources
    }

    /// Compile: topological order from write→read deps, lifetimes, alias slots.
    pub fn compile(&self) -> Result<CompiledGraph, GraphError> {
        let n = self.passes.len();
        let mut writers: HashMap<ResourceId, Vec<PassId>> = HashMap::new();
        for (i, pass) in self.passes.iter().enumerate() {
            let pid = PassId(i as u32);
            for r in &pass.resources {
                if r.usage == ResourceUsage::Write {
                    writers.entry(r.id).or_default().push(pid);
                }
            }
        }

        // Edge: writer → reader (reader depends on writer).
        let mut adj: Vec<Vec<PassId>> = vec![Vec::new(); n];
        let mut indegree = vec![0u32; n];
        for (i, pass) in self.passes.iter().enumerate() {
            let reader = PassId(i as u32);
            for r in &pass.resources {
                if r.usage != ResourceUsage::Read {
                    continue;
                }
                if let Some(ws) = writers.get(&r.id) {
                    for &w in ws {
                        if w.0 != reader.0 {
                            adj[w.0 as usize].push(reader);
                            indegree[reader.0 as usize] += 1;
                        }
                    }
                }
            }
        }

        // Kahn topological sort; stable by declaration order when ties.
        let mut queue: Vec<PassId> = (0..n)
            .filter(|&i| indegree[i] == 0)
            .map(|i| PassId(i as u32))
            .collect();
        queue.sort_by_key(|p| p.0);
        let mut order = Vec::with_capacity(n);
        while !queue.is_empty() {
            let p = queue.remove(0);
            order.push(p);
            let mut next: Vec<PassId> = adj[p.0 as usize].clone();
            next.sort_by_key(|x| x.0);
            for npass in next {
                let i = npass.0 as usize;
                indegree[i] -= 1;
                if indegree[i] == 0 {
                    queue.push(npass);
                    queue.sort_by_key(|x| x.0);
                }
            }
        }
        if order.len() != n {
            return Err(GraphError::Cycle);
        }

        let lifetimes = compute_lifetimes(&self.passes, &order, self.resources.len());
        let (slot_of, slot_count) = assign_alias_slots(&self.resources, &lifetimes);

        Ok(CompiledGraph {
            pass_order: order,
            lifetimes,
            slot_of,
            slot_count,
            pass_names: self.passes.iter().map(|p| p.name.clone()).collect(),
            resources: self.resources.clone(),
        })
    }
}

/// Lifetime of a resource in compiled pass-order indices `[first, last]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Lifetime {
    pub first: usize,
    pub last: usize,
}

/// Result of compiling a [`RenderGraph`].
#[derive(Debug, Clone)]
pub struct CompiledGraph {
    pub pass_order: Vec<PassId>,
    pub lifetimes: Vec<Option<Lifetime>>,
    /// Resource index → physical slot (alias group). External/surface may be None.
    pub slot_of: Vec<Option<usize>>,
    pub slot_count: usize,
    pub pass_names: Vec<String>,
    pub resources: Vec<ResourceDesc>,
}

impl CompiledGraph {
    pub fn ordered_pass_names(&self) -> Vec<&str> {
        self.pass_order
            .iter()
            .map(|id| self.pass_names[id.0 as usize].as_str())
            .collect()
    }

    /// Two resources share storage when assigned the same slot.
    pub fn aliases(&self, a: ResourceId, b: ResourceId) -> bool {
        match (
            self.slot_of.get(a.0 as usize).copied().flatten(),
            self.slot_of.get(b.0 as usize).copied().flatten(),
        ) {
            (Some(sa), Some(sb)) => sa == sb,
            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    Cycle,
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphError::Cycle => write!(f, "render graph contains a cycle"),
        }
    }
}

impl std::error::Error for GraphError {}

fn compute_lifetimes(
    passes: &[PassDecl],
    order: &[PassId],
    resource_count: usize,
) -> Vec<Option<Lifetime>> {
    let mut lifetimes = vec![None; resource_count];
    for (step, pid) in order.iter().enumerate() {
        let pass = &passes[pid.0 as usize];
        for r in &pass.resources {
            let idx = r.id.0 as usize;
            match &mut lifetimes[idx] {
                None => {
                    lifetimes[idx] = Some(Lifetime {
                        first: step,
                        last: step,
                    });
                }
                Some(lt) => {
                    lt.last = step;
                }
            }
        }
    }
    lifetimes
}

fn resources_compatible(a: &ResourceDesc, b: &ResourceDesc) -> bool {
    a.kind == b.kind
}

fn lifetimes_overlap(a: Lifetime, b: Lifetime) -> bool {
    a.first <= b.last && b.first <= a.last
}

/// Greedy aliasing: same-shaped non-overlapping transients share a slot.
fn assign_alias_slots(
    resources: &[ResourceDesc],
    lifetimes: &[Option<Lifetime>],
) -> (Vec<Option<usize>>, usize) {
    let mut slot_of = vec![None; resources.len()];
    // slot → (desc index of representative, lifetime)
    let mut slots: Vec<(usize, Lifetime)> = Vec::new();

    for (i, desc) in resources.iter().enumerate() {
        if !desc.transient {
            continue;
        }
        let Some(lt) = lifetimes[i] else {
            continue;
        };
        let mut placed = false;
        for (slot_idx, (rep, slot_lt)) in slots.iter_mut().enumerate() {
            if resources_compatible(&resources[*rep], desc) && !lifetimes_overlap(*slot_lt, lt) {
                // Extend slot lifetime to cover both.
                slot_lt.first = slot_lt.first.min(lt.first);
                slot_lt.last = slot_lt.last.max(lt.last);
                slot_of[i] = Some(slot_idx);
                placed = true;
                break;
            }
        }
        if !placed {
            let slot_idx = slots.len();
            slots.push((i, lt));
            slot_of[i] = Some(slot_idx);
        }
    }

    // Non-transient / unused: each gets its own dedicated slot if used.
    for (i, desc) in resources.iter().enumerate() {
        if slot_of[i].is_some() {
            continue;
        }
        if lifetimes[i].is_none() {
            continue;
        }
        if desc.transient {
            // Should have been assigned; safety net.
            let slot_idx = slots.len();
            slots.push((i, lifetimes[i].unwrap()));
            slot_of[i] = Some(slot_idx);
        } else {
            let slot_idx = slots.len();
            slots.push((i, lifetimes[i].unwrap()));
            slot_of[i] = Some(slot_idx);
        }
    }

    (slot_of, slots.len())
}

#[cfg(test)]
#[path = "graph_tests.rs"]
mod tests;
