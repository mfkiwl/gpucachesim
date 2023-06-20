use super::{interconn as ic, mem_fetch, stats::Stats, MockSimulator, Packet, SIMTCore};
use crate::config::GPUConfig;
use console::style;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
// pub struct SIMTCoreCluster {
pub struct SIMTCoreCluster<I> {
    pub cluster_id: usize,
    // pub cores: Mutex<Vec<SIMTCore>>,
    // pub cores: Mutex<Vec<SIMTCore<'a>>>,
    pub cores: Mutex<Vec<SIMTCore<I>>>,
    // pub cores: Mutex<Vec<SIMTCore<ic::CoreMemoryInterface>>>,
    pub config: Arc<GPUConfig>,
    pub stats: Arc<Mutex<Stats>>,

    pub interconn: Arc<I>,

    pub core_sim_order: Vec<usize>,
    pub block_issue_next_core: Mutex<usize>,
    pub response_fifo: VecDeque<mem_fetch::MemFetch>,
}

// impl super::MemFetchInterconnect for SIMTCoreCluster {
//     fn full(&self, size: usize, write: bool) -> bool {
//         self.cluster.interconn_injection_buffer_full(size, write)
//     }
//
//     fn push(&mut self, fetch: mem_fetch::MemFetch) {
//         // self.core.inc_simt_to_mem(fetch->get_num_flits(true));
//         self.cluster.interconn_inject_request_packet(fetch);
//     }
// }

// impl SIMTCoreCluster {
// impl<'a> SIMTCoreCluster<'a> {
impl<I> SIMTCoreCluster<I>
where
    // I: ic::MemFetchInterface + 'static,
    I: ic::Interconnect<Packet> + 'static,
    // I: ic::Interconnect<Packet>,
{
    pub fn new(
        cluster_id: usize,
        interconn: Arc<I>,
        stats: Arc<Mutex<Stats>>,
        config: Arc<GPUConfig>,
    ) -> Self {
        // let mut core_sim_order = Vec::new();
        // let cores: Vec<_> = (0..config.num_cores_per_simt_cluster)
        //     .map(|core_id| {
        //         core_sim_order.push(core_id);
        //         let id = config.global_core_id(cluster_id, core_id);
        //         SIMTCore::new(id, cluster_id, Arc::new(self), stats.clone(), config.clone())
        //     })
        //     .collect();

        //     unsigned sid = m_config->cid_to_sid(i, m_cluster_id);
        //     m_core[i] = new trace_shader_core_ctx(m_gpu, this, sid, m_cluster_id,
        //                                           m_config, m_mem_config, m_stats);

        let num_cores = config.num_cores_per_simt_cluster;
        let block_issue_next_core = Mutex::new(num_cores - 1);
        let mut cluster = Self {
            cluster_id,
            config: config.clone(),
            stats: stats.clone(),
            interconn: interconn.clone(),
            // cores: Mutex::new(cores),
            cores: Mutex::new(Vec::new()),
            core_sim_order: Vec::new(),
            block_issue_next_core,
            response_fifo: VecDeque::new(),
        };
        let cores = (0..num_cores)
            .map(|core_id| {
                cluster.core_sim_order.push(core_id);
                let id = config.global_core_id(cluster_id, core_id);
                SIMTCore::new(
                    id,
                    cluster_id,
                    // Arc::new(cluster),
                    interconn.clone(),
                    stats.clone(),
                    config.clone(),
                )
            })
            .collect();
        cluster.cores = Mutex::new(cores);
        cluster.reinit();
        cluster
    }

    fn reinit(&mut self) {
        for core in self.cores.lock().unwrap().iter_mut() {
            core.reinit(0, self.config.max_threads_per_core, true);
        }
    }

    pub fn num_active_sms(&self) -> usize {
        self.cores
            .lock()
            .unwrap()
            .iter()
            .filter(|c| c.active())
            .count()
    }

    pub fn not_completed(&self) -> usize {
        self.cores
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.not_completed())
            .sum()
        // not_completed += m_core[i]->get_not_completed();
        // todo!("cluster: not completed");
        // true
    }

    pub fn warp_waiting_at_barrier(&self, warp_id: usize) -> bool {
        todo!("cluster: warp_waiting_at_barrier");
        // self.barriers.warp_waiting_at_barrier(warp_id)
    }

    pub fn warp_waiting_at_mem_barrier(&self, warp_id: usize) -> bool {
        todo!("cluster: warp_waiting_at_mem_barrier");
        // if (!m_warp[warp_id]->get_membar()) return false;
        // if (!m_scoreboard->pendingWrites(warp_id)) {
        //   m_warp[warp_id]->clear_membar();
        //   if (m_gpu->get_config().flush_l1()) {
        //     // Mahmoud fixed this on Nov 2019
        //     // Invalidate L1 cache
        //     // Based on Nvidia Doc, at MEM barrier, we have to
        //     //(1) wait for all pending writes till they are acked
        //     //(2) invalidate L1 cache to ensure coherence and avoid reading stall data
        //     cache_invalidate();
        //     // TO DO: you need to stall the SM for 5k cycles.
        //   }
        //   return false;
        // }
        // return true;
    }

    // pub fn interconn_inject_request_packet(&mut self, mut fetch: mem_fetch::MemFetch) {
    //     todo!(
    //         "cluster {}: interconn_inject_request_packet",
    //         self.cluster_id
    //     );
    //     {
    //         let mut stats = self.stats.lock().unwrap();
    //         if fetch.is_write() {
    //             stats.num_mem_write += 1;
    //         } else {
    //             stats.num_mem_read += 1;
    //         }
    //
    //         match fetch.access_kind() {
    //             mem_fetch::AccessKind::CONST_ACC_R => {
    //                 stats.num_mem_const += 1;
    //             }
    //             mem_fetch::AccessKind::TEXTURE_ACC_R => {
    //                 stats.num_mem_texture += 1;
    //             }
    //             mem_fetch::AccessKind::GLOBAL_ACC_R => {
    //                 stats.num_mem_read_global += 1;
    //             }
    //             mem_fetch::AccessKind::GLOBAL_ACC_W => {
    //                 stats.num_mem_write_global += 1;
    //             }
    //             mem_fetch::AccessKind::LOCAL_ACC_R => {
    //                 stats.num_mem_read_local += 1;
    //             }
    //             mem_fetch::AccessKind::LOCAL_ACC_W => {
    //                 stats.num_mem_write_local += 1;
    //             }
    //             mem_fetch::AccessKind::INST_ACC_R => {
    //                 stats.num_mem_read_inst += 1;
    //             }
    //             mem_fetch::AccessKind::L1_WRBK_ACC => {
    //                 stats.num_mem_write_global += 1;
    //             }
    //             mem_fetch::AccessKind::L2_WRBK_ACC => {
    //                 stats.num_mem_l2_writeback += 1;
    //             }
    //             mem_fetch::AccessKind::L1_WR_ALLOC_R => {
    //                 stats.num_mem_l1_write_allocate += 1;
    //             }
    //             mem_fetch::AccessKind::L2_WR_ALLOC_R => {
    //                 stats.num_mem_l2_write_allocate += 1;
    //             }
    //             _ => {}
    //         }
    //     }
    //
    //     // The packet size varies depending on the type of request:
    //     // - For write request and atomic request, the packet contains the data
    //     // - For read request (i.e. not write nor atomic), the packet only has control
    //     // metadata
    //     let packet_size = if fetch.is_write() && fetch.is_atomic() {
    //         fetch.control_size
    //     } else {
    //         fetch.data_size
    //     };
    //     // m_stats->m_outgoing_traffic_stats->record_traffic(mf, packet_size);
    //     let dest = fetch.sub_partition_id();
    //     fetch.status = mem_fetch::Status::IN_ICNT_TO_MEM;
    //
    //     // if !fetch.is_write() && !fetch.is_atomic() {
    //     self.interconn.push(
    //         self.cluster_id,
    //         self.config.mem_id_to_device_id(dest as usize),
    //         fetch,
    //         packet_size,
    //     );
    // }

    pub fn interconn_cycle(&mut self) {
        use mem_fetch::AccessKind;

        println!(
            "cluster {}: {} (response fifo size={})",
            self.cluster_id,
            style("interconn cycle").cyan(),
            self.response_fifo.len(),
        );

        if let Some(fetch) = self.response_fifo.front() {
            let core_id = self.config.global_core_id_to_core_id(fetch.core_id);
            // debug_assert_eq!(core_id, fetch.cluster_id);
            let mut cores = self.cores.lock().unwrap();
            let core = &mut cores[core_id];
            match *fetch.access_kind() {
                AccessKind::INST_ACC_R => {
                    // instruction fetch response
                    // if !core.fetch_unit_response_buffer_full() {
                    let fetch = self.response_fifo.pop_front().unwrap();
                    core.accept_fetch_response(fetch);
                    // }
                }
                _ => {
                    // panic!("got data response");
                    // data response
                    if !core.ldst_unit_response_buffer_full() {
                        let fetch = self.response_fifo.pop_front().unwrap();
                        // m_memory_stats->memlatstat_read_done(mf);
                        core.accept_ldst_unit_response(fetch);
                    }
                }
            }
        }
        let eject_buffer_size = self.config.num_cluster_ejection_buffer_size;
        if self.response_fifo.len() >= eject_buffer_size {
            return;
        }

        let Some(Packet::Fetch(mut fetch)) = self.interconn.pop(self.cluster_id) else {
            return;
        };
        println!(
            "{} addr={} kind={:?}",
            style(format!(
                "got fetch from interconn: {:?} ",
                fetch.access_kind()
            ))
            .cyan(),
            fetch.addr(),
            fetch.kind
        );
        // dbg!(&fetch.access_kind());
        // dbg!(&fetch.addr());
        // dbg!(&fetch.kind);
        debug_assert_eq!(fetch.cluster_id, self.cluster_id);
        // debug_assert!(matches!(
        //     fetch.kind,
        //     mem_fetch::Kind::READ_REPLY | mem_fetch::Kind::WRITE_ACK
        // ));

        // The packet size varies depending on the type of request:
        // - For read request and atomic request, the packet contains the data
        // - For write-ack, the packet only has control metadata
        let packet_size = if fetch.is_write() {
            fetch.control_size
        } else {
            fetch.data_size
        };
        // m_stats->m_incoming_traffic_stats->record_traffic(mf, packet_size);
        fetch.status = mem_fetch::Status::IN_CLUSTER_TO_SHADER_QUEUE;
        self.response_fifo.push_back(fetch.clone());

        // m_stats->n_mem_to_simt[m_cluster_id] += mf->get_num_flits(false);
    }

    pub fn cache_flush(&mut self) {
        let mut cores = self.cores.lock().unwrap();
        for core in cores.iter_mut() {
            core.cache_flush();
        }
    }

    pub fn cache_invalidate(&mut self) {
        let mut cores = self.cores.lock().unwrap();
        for core in cores.iter_mut() {
            core.cache_invalidate();
        }
    }

    pub fn cycle(&mut self) {
        let mut cores = self.cores.lock().unwrap();
        for core in cores.iter_mut() {
            core.cycle()
        }
    }

    pub fn issue_block_to_core(&self, sim: &MockSimulator<I>) -> usize {
        println!("cluster {}: issue block 2 core", self.cluster_id);
        let mut num_blocks_issued = 0;

        let mut block_issue_next_core = self.block_issue_next_core.lock().unwrap();
        let mut cores = self.cores.lock().unwrap();
        let num_cores = cores.len();
        // dbg!(&sim.select_kernel());

        for (i, core) in cores.iter_mut().enumerate() {
            // debug_assert_eq!(i, core.id);
            let core_id = (i + *block_issue_next_core + 1) % num_cores;
            let mut kernel = None;
            if self.config.concurrent_kernel_sm {
                // always select latest issued kernel
                kernel = sim.select_kernel()
            } else {
                if core
                    .inner
                    .current_kernel
                    .as_ref()
                    .map(|current| !current.no_more_blocks_to_run())
                    .unwrap_or(true)
                {
                    // wait until current kernel finishes
                    if core.inner.num_active_warps == 0 {
                        kernel = sim.select_kernel();
                        if let Some(k) = kernel {
                            core.set_kernel(k.clone());
                        }
                    }
                }
            }
            println!(
                "core {}-{}: current kernel {}",
                self.cluster_id,
                core.inner.core_id,
                &core.inner.current_kernel.is_some()
            );
            println!(
                "core {}-{}: selected kernel {:?}",
                self.cluster_id,
                core.inner.core_id,
                kernel.as_ref().map(|k| k.name())
            );
            if let Some(kernel) = kernel {
                // dbg!(&kernel.no_more_blocks_to_run());
                // dbg!(&core.can_issue_block(&*kernel));
                if !kernel.no_more_blocks_to_run() && core.can_issue_block(&*kernel) {
                    core.issue_block(kernel.clone());
                    num_blocks_issued += 1;
                    *block_issue_next_core = core_id;
                    break;
                }
            }
        }
        num_blocks_issued

        // pub fn id(&self) -> (usize, usize) {
        //         self.id,
        //         core.id,
        //
        // }
        //       unsigned num_blocks_issued = 0;
        // for (unsigned i = 0; i < m_config->n_simt_cores_per_cluster; i++) {
        //   unsigned core =
        //       (i + m_cta_issue_next_core + 1) % m_config->n_simt_cores_per_cluster;
        //
        //   kernel_info_t *kernel;
        //   // Jin: fetch kernel according to concurrent kernel setting
        //   if (m_config->gpgpu_concurrent_kernel_sm) {  // concurrent kernel on sm
        //     // always select latest issued kernel
        //     kernel_info_t *k = m_gpu->select_kernel();
        //     kernel = k;
        //   } else {
        //     // first select core kernel, if no more cta, get a new kernel
        //     // only when core completes
        //     kernel = m_core[core]->get_kernel();
        //     if (!m_gpu->kernel_more_cta_left(kernel)) {
        //       // wait till current kernel finishes
        //       if (m_core[core]->get_not_completed() == 0) {
        //         kernel_info_t *k = m_gpu->select_kernel();
        //         if (k) m_core[core]->set_kernel(k);
        //         kernel = k;
        //       }
        //     }
        //   }
        //
        //   if (m_gpu->kernel_more_cta_left(kernel) &&
        //       //            (m_core[core]->get_n_active_cta() <
        //       //            m_config->max_cta(*kernel)) ) {
        //       m_core[core]->can_issue_1block(*kernel)) {
        //     m_core[core]->issue_block2core(*kernel);
        //     num_blocks_issued++;
        //     m_cta_issue_next_core = core;
        //     break;
        //   }
        // }
        // return num_blocks_issued;
    }
}
