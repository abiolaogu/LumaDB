use std::sync::atomic::{AtomicUsize, Ordering};
use std::collections::HashMap;

/// Logical Core ID
pub type CoreId = usize;

/// CPU Topology Manager
pub struct CpuTopology {
    physical_cores: usize,
    logical_cores: usize,
    numa_nodes: usize,
    core_map: HashMap<CoreId, NumaNodeId>,
}

pub type NumaNodeId = i32;

impl CpuTopology {
    pub fn detect() -> Self {
        // In a real impl, we'd use `hwloc` or `libc` to query topology.
        // For now, we fallback to logical core count.
        let logical_cores = num_cpus::get();
        let physical_cores = num_cpus::get_physical();
        
        Self {
            physical_cores,
            logical_cores,
            numa_nodes: 1, // Default to UMA
            core_map: HashMap::new(),
        }
    }

    pub fn logical_cores(&self) -> usize {
        self.logical_cores
    }
}

/// Pin current thread to a specific core
pub fn pin_to_core(core_id: CoreId) {
    #[cfg(target_os = "linux")]
    {
        use libc::{cpu_set_t, sched_setaffinity, CPU_SET, CPU_ZERO};
        use std::mem;

        unsafe {
            let mut set: cpu_set_t = mem::zeroed();
            CPU_ZERO(&mut set);
            CPU_SET(core_id, &mut set);
            
            // 0 = current thread
            let ret = sched_setaffinity(0, mem::size_of::<cpu_set_t>(), &set);
            if ret != 0 {
                eprintln!("Failed to pin thread to core {}", core_id);
            }
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        // No-op on MacOS/Windows for now in this impl
        // println!("Skipping thread pinning (not linux)");
    }
}

pub struct CoreAffinity {
    core_id: CoreId,
}

impl CoreAffinity {
    pub fn new(core_id: CoreId) -> Self {
        Self { core_id }
    }
    
    pub fn apply(&self) {
        pin_to_core(self.core_id);
    }
}
