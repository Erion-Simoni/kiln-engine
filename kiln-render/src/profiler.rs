//! GPU timestamp queries for per-pass cost.

use wgpu::{BufferUsages, CommandEncoder, Device, QuerySet, Queue};

/// Timing for one named pass (nanoseconds of GPU time).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassTiming {
    pub name: String,
    pub duration_ns: u64,
}

impl PassTiming {
    pub fn duration_us(&self) -> f64 {
        self.duration_ns as f64 / 1_000.0
    }

    pub fn duration_ms(&self) -> f64 {
        self.duration_ns as f64 / 1_000_000.0
    }
}

/// Records GPU timestamps around passes when `TIMESTAMP_QUERY` is available.
pub struct GpuProfiler {
    supported: bool,
    query_set: Option<QuerySet>,
    resolve_buffer: Option<wgpu::Buffer>,
    readback_buffer: Option<wgpu::Buffer>,
    /// Capacity in timestamp slots (2 per pass: begin + end).
    capacity: u32,
    /// Pass names for the in-flight frame (index → name).
    pending_names: Vec<String>,
    /// Next timestamp index to write.
    next_ts: u32,
    /// Last resolved timings (one frame delayed typical).
    last_timings: Vec<PassTiming>,
    timestamp_period: f32,
    /// Frame waiting on readback map.
    pending_resolve: bool,
}

impl GpuProfiler {
    /// Create profiler; requests no device features (caller must enable TIMESTAMP_QUERY).
    pub fn new(device: &Device, queue: &Queue, max_passes: u32) -> Self {
        let features = device.features();
        let need =
            wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        let supported = features.contains(need);
        let capacity = max_passes.saturating_mul(2).max(2);

        if !supported {
            return Self {
                supported: false,
                query_set: None,
                resolve_buffer: None,
                readback_buffer: None,
                capacity,
                pending_names: Vec::new(),
                next_ts: 0,
                last_timings: Vec::new(),
                timestamp_period: 1.0,
                pending_resolve: false,
            };
        }

        let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("kiln_gpu_timestamps"),
            ty: wgpu::QueryType::Timestamp,
            count: capacity,
        });

        let bytes = (capacity as u64) * 8;
        let resolve_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kiln_ts_resolve"),
            size: bytes,
            usage: BufferUsages::QUERY_RESOLVE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("kiln_ts_readback"),
            size: bytes,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        Self {
            supported: true,
            query_set: Some(query_set),
            resolve_buffer: Some(resolve_buffer),
            readback_buffer: Some(readback_buffer),
            capacity,
            pending_names: Vec::new(),
            next_ts: 0,
            last_timings: Vec::new(),
            timestamp_period: queue.get_timestamp_period(),
            pending_resolve: false,
        }
    }

    pub fn supported(&self) -> bool {
        self.supported
    }

    pub fn timings(&self) -> &[PassTiming] {
        &self.last_timings
    }

    /// Start a profiled frame (clears pending pass list).
    pub fn begin_frame(&mut self) {
        if !self.supported {
            return;
        }
        self.pending_names.clear();
        self.next_ts = 0;
    }

    /// Write begin timestamp; returns index used for [`end_pass`].
    pub fn begin_pass(&mut self, encoder: &mut CommandEncoder, name: &str) -> Option<u32> {
        if !self.supported {
            return None;
        }
        if self.next_ts + 1 >= self.capacity {
            return None;
        }
        let begin = self.next_ts;
        if let Some(qs) = self.query_set.as_ref() {
            encoder.write_timestamp(qs, begin);
        }
        self.next_ts += 1;
        self.pending_names.push(name.to_string());
        Some(begin)
    }

    /// Write end timestamp for a pass started with [`begin_pass`].
    pub fn end_pass(&mut self, encoder: &mut CommandEncoder, begin_index: Option<u32>) {
        if !self.supported {
            return;
        }
        let Some(_begin) = begin_index else {
            return;
        };
        if self.next_ts >= self.capacity {
            return;
        }
        if let Some(qs) = self.query_set.as_ref() {
            encoder.write_timestamp(qs, self.next_ts);
        }
        self.next_ts += 1;
    }

    /// Resolve queries into the readback buffer. Call before `queue.submit`.
    pub fn resolve(&mut self, encoder: &mut CommandEncoder) {
        if !self.supported || self.next_ts == 0 {
            return;
        }
        let Some(qs) = self.query_set.as_ref() else {
            return;
        };
        let Some(resolve) = self.resolve_buffer.as_ref() else {
            return;
        };
        let Some(readback) = self.readback_buffer.as_ref() else {
            return;
        };
        encoder.resolve_query_set(qs, 0..self.next_ts, resolve, 0);
        encoder.copy_buffer_to_buffer(resolve, 0, readback, 0, self.next_ts as u64 * 8);
        self.pending_resolve = true;
    }

    /// Map readback asynchronously and update [`timings`]. Call after submit.
    pub fn poll_timings(&mut self, device: &Device) {
        if !self.supported || !self.pending_resolve {
            return;
        }
        let Some(readback) = self.readback_buffer.as_ref() else {
            return;
        };
        let ts_count = self.next_ts;
        let names = self.pending_names.clone();
        let period = self.timestamp_period;

        let slice = readback.slice(..(ts_count as u64 * 8));
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |r| {
            let _ = tx.send(r);
        });
        device.poll(wgpu::Maintain::Wait);
        if rx.recv().ok().and_then(|r| r.ok()).is_none() {
            return;
        }

        let data = slice.get_mapped_range();
        let mut timings = Vec::new();
        for (i, name) in names.iter().enumerate() {
            let begin_i = i * 2;
            let end_i = begin_i + 1;
            if end_i >= ts_count as usize {
                break;
            }
            let begin = read_u64(&data, begin_i);
            let end = read_u64(&data, end_i);
            let ticks = end.saturating_sub(begin);
            let duration_ns = (ticks as f64 * period as f64) as u64;
            timings.push(PassTiming {
                name: name.clone(),
                duration_ns,
            });
        }
        drop(data);
        readback.unmap();

        self.last_timings = timings;
        self.pending_resolve = false;
    }
}

fn read_u64(data: &[u8], index: usize) -> u64 {
    let start = index * 8;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&data[start..start + 8]);
    u64::from_le_bytes(buf)
}

/// Features the renderer should request when available for profiling.
///
/// `write_timestamp` on a command encoder needs `TIMESTAMP_QUERY_INSIDE_ENCODERS`
/// in addition to `TIMESTAMP_QUERY` (wgpu 22+).
pub fn requested_profiler_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    let available = adapter.features();
    let need = wgpu::Features::TIMESTAMP_QUERY | wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
    if available.contains(need) {
        need
    } else {
        wgpu::Features::empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_timing_units() {
        let t = PassTiming {
            name: "main".into(),
            duration_ns: 2_500_000,
        };
        assert!((t.duration_us() - 2500.0).abs() < 0.01);
        assert!((t.duration_ms() - 2.5).abs() < 0.01);
    }
}
