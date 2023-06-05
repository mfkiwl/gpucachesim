use crate::config;
use crate::ported::{
    self, address, cache, interconn as ic, mem_fetch, mshr, stats::Stats, tag_array,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Metadata for port bandwidth management
#[derive(Clone)]
pub struct BandwidthManager {
    config: Arc<config::CacheConfig>,

    /// number of cycle that the data port remains used
    data_port_occupied_cycles: usize,
    /// number of cycle that the fill port remains used
    fill_port_occupied_cycles: usize,
}

impl std::fmt::Debug for BandwidthManager {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("BandwidthManager")
            .field("data_port_occupied_cycles", &self.data_port_occupied_cycles)
            .field("fill_port_occupied_cycles", &self.fill_port_occupied_cycles)
            .field("data_port_free", &self.data_port_free())
            .field("fill_port_free", &self.fill_port_free())
            .finish()
    }
}

impl BandwidthManager {
    /// Create a new bandwidth manager from config
    pub fn new(config: Arc<config::CacheConfig>) -> Self {
        Self {
            config,
            data_port_occupied_cycles: 0,
            fill_port_occupied_cycles: 0,
        }
    }

    /// Use the data port based on the outcome and
    /// events generated by the mem_fetch request
    pub fn use_data_port(
        &mut self,
        fetch: &mem_fetch::MemFetch,
        outcome: cache::RequestStatus,
        events: Vec<cache::Event>,
    ) {
        todo!("bandwidth: use data port");
    }

    /// Use the fill port
    pub fn use_fill_port(&mut self, fetch: &mem_fetch::MemFetch) {
        // assume filling the entire line with the
        // returned request
        let fill_cycles = self.config.atom_size() / self.config.data_port_width();
        self.fill_port_occupied_cycles += fill_cycles;
        // todo!("bandwidth: use fill port");
    }

    /// Free up used ports.
    ///
    /// This is called every cache cycle.
    pub fn replenish_port_bandwidth(&mut self) {
        if self.data_port_occupied_cycles > 0 {
            self.data_port_occupied_cycles -= 1;
        }
        debug_assert!(self.data_port_occupied_cycles >= 0);

        if self.fill_port_occupied_cycles > 0 {
            self.fill_port_occupied_cycles -= 1;
        }
        debug_assert!(self.fill_port_occupied_cycles >= 0);
        // todo!("bandwidth: replenish port bandwidth");
    }

    /// Query for data port availability
    pub fn data_port_free(&self) -> bool {
        self.data_port_occupied_cycles == 0
        // todo!("bandwidth: data port free");
    }
    /// Query for fill port availability
    pub fn fill_port_free(&self) -> bool {
        self.fill_port_occupied_cycles == 0
        // todo!("bandwidth: data port free");
    }
}

/// Base cache
/// Implements common functions for read_only_cache and data_cache
/// Each subclass implements its own 'access' function
#[derive()]
pub struct Base<I>
// where
//     I: ic::MemPort,
{
    pub core_id: usize,
    pub cluster_id: usize,

    pub stats: Arc<Mutex<Stats>>,
    pub config: Arc<config::GPUConfig>,
    pub cache_config: Arc<config::CacheConfig>,

    pub miss_queue: VecDeque<mem_fetch::MemFetch>,
    pub miss_queue_status: mem_fetch::Status,
    pub mshrs: mshr::MshrTable,
    pub tag_array: tag_array::TagArray<()>,
    pub mem_port: Arc<I>,

    // /// Specifies type of write allocate request
    // ///
    // /// (e.g., L1 or L2)
    // write_alloc_type: mem_fetch::AccessKind,
    //
    // /// Specifies type of writeback request
    // ///
    // /// (e.g., L1 or L2)
    // write_back_type: mem_fetch::AccessKind,
    pub bandwidth: BandwidthManager,
}

impl<I> std::fmt::Debug for Base<I> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Base")
            .field("core_id", &self.core_id)
            .field("cluster_id", &self.cluster_id)
            .field("miss_queue", &self.miss_queue)
            .finish()
    }
}

impl<I> Base<I> {
    pub fn new(
        core_id: usize,
        cluster_id: usize,
        // tag_array: tag_array::TagArray<()>,
        mem_port: Arc<I>,
        stats: Arc<Mutex<Stats>>,
        config: Arc<config::GPUConfig>,
        cache_config: Arc<config::CacheConfig>,
    ) -> Self {
        // for now we initialize the tag array and mshr

        // m_tag_array(new tag_array(config, core_id, type_id)),
        let tag_array = tag_array::TagArray::new(core_id, 0, cache_config.clone());

        // m_mshrs(config.m_mshr_entries, config.m_mshr_max_merge),
        debug_assert!(matches!(
            cache_config.mshr_kind,
            config::MshrKind::ASSOC | config::MshrKind::SECTOR_ASSOC
        ));
        let mshrs = mshr::MshrTable::new(cache_config.mshr_entries, cache_config.mshr_max_merge);

        let bandwidth = BandwidthManager::new(cache_config.clone());
        Self {
            core_id,
            cluster_id,
            tag_array,
            mshrs,
            mem_port,
            stats,
            config,
            cache_config,
            bandwidth,
            miss_queue: VecDeque::new(),
            miss_queue_status: mem_fetch::Status::INITIALIZED,
            // write_alloc_type: mem_fetch::AccessKind::L1_WR_ALLOC_R,
            // write_back_type: mem_fetch::AccessKind::L1_WRBK_ACC,
        }
    }

    /// Checks whether this request can be handled in this cycle.
    ///
    /// `n` equals the number of misses to be handled on
    /// this cycle.
    pub fn miss_queue_can_fit(&self, n: usize) -> bool {
        self.miss_queue.len() + n < self.cache_config.miss_queue_size
    }

    /// Checks whether the miss queue is full.
    ///
    /// This leads to misses not being handled in this cycle.
    pub fn miss_queue_full(&self) -> bool {
        self.miss_queue.len() >= self.cache_config.miss_queue_size
    }

    /// Checks if fetch is waiting to be filled
    /// by lower memory level
    pub fn waiting_for_fill(&self, fetch: &mem_fetch::MemFetch) {
        // extra_mf_fields_lookup::iterator e = m_extra_mf_fields.find(mf);
        // return e != m_extra_mf_fields.end();
        todo!("base cache: waiting for fill");
    }

    /// Are any (accepted) accesses that had to wait for memory now ready?
    ///
    /// Note: does not include accesses that "HIT"
    pub fn has_ready_accesses(&self) -> bool {
        self.mshrs.has_ready_accesses()
    }

    /// Pop next ready access
    ///
    /// Note: does not include accesses that "HIT"
    pub fn next_access(&mut self) -> Option<mem_fetch::MemFetch> {
        self.mshrs.next_access()
    }

    /// Flush all entries in cache
    fn flush(&mut self) {
        self.tag_array.flush();
    }

    /// Invalidate all entries in cache
    fn invalidate(&mut self) {
        self.tag_array.invalidate();
    }

    /// Read miss handler.
    ///
    /// Check MSHR hit or MSHR available
    pub fn send_read_request(
        &mut self,
        addr: address,
        block_addr: u64,
        cache_index: Option<usize>,
        mut fetch: mem_fetch::MemFetch,
        time: usize,
        // events: &mut Option<Vec<cache::Event>>,
        // events: &mut Option<&mut Vec<cache::Event>>,
        read_only: bool,
        write_allocate: bool,
    ) -> (bool, bool, Option<tag_array::EvictedBlockInfo>) {
        let mut should_miss = false;
        let mut writeback = false;
        let mut evicted = None;

        let mshr_addr = self.cache_config.mshr_addr(fetch.addr());
        let mshr_hit = self.mshrs.probe(mshr_addr);
        let mshr_full = self.mshrs.full(mshr_addr);
        let mut cache_index = cache_index.expect("cache index");

        if mshr_hit && !mshr_full {
            if read_only {
                self.tag_array.access(block_addr, time, &fetch);
            } else {
                tag_array::AccessStatus {
                    writeback,
                    evicted,
                    ..
                } = self.tag_array.access(block_addr, time, &fetch);
            }

            self.mshrs.add(mshr_addr, fetch.clone());
            // m_stats.inc_stats(mf->get_access_type(), MSHR_HIT);
            should_miss = true;
        } else if !mshr_hit && !mshr_full && !self.miss_queue_full() {
            if read_only {
                self.tag_array.access(block_addr, time, &fetch);
            } else {
                tag_array::AccessStatus {
                    writeback,
                    evicted,
                    ..
                } = self.tag_array.access(block_addr, time, &fetch);
            }

            // m_extra_mf_fields[mf] = extra_mf_fields(
            //     mshr_addr, mf->get_addr(), cache_index, mf->get_data_size(), m_config);
            fetch.data_size = self.cache_config.atom_size() as u32;
            fetch.set_addr(mshr_addr);

            self.mshrs.add(mshr_addr, fetch.clone());
            self.miss_queue.push_back(fetch.clone());
            fetch.set_status(self.miss_queue_status, time);
            if !write_allocate {
                // if let Some(events) = events {
                //     let event = cache::Event::new(cache::EventKind::READ_REQUEST_SENT);
                //     events.push(event);
                // }
            }

            should_miss = true;
        } else if mshr_hit && mshr_full {
            // m_stats.inc_fail_stats(fetch.access_kind(), MSHR_MERGE_ENRTY_FAIL);
        } else if !mshr_hit && mshr_full {
            // m_stats.inc_fail_stats(fetch.access_kind(), MSHR_ENRTY_FAIL);
        } else {
            panic!("mshr full?");
        }
        (should_miss, write_allocate, evicted)
    }

    // /// Sends write request to lower level memory (write or writeback)
    // pub fn send_write_request(
    //     &mut self,
    //     mut fetch: mem_fetch::MemFetch,
    //     request: cache::Event,
    //     time: usize,
    //     // events: &Option<&mut Vec<cache::Event>>,
    // ) {
    //     println!("data_cache::send_write_request(...)");
    //     // if let Some(events) = events {
    //     //     events.push(request);
    //     // }
    //     fetch.set_status(self.miss_queue_status, time);
    //     self.miss_queue.push_back(fetch);
    // }

    // /// Base read miss
    // ///
    // /// Send read request to lower level memory and perform
    // /// write-back as necessary.
    // fn read_miss(
    //     &mut self,
    //     addr: address,
    //     cache_index: Option<usize>,
    //     // cache_index: usize,
    //     fetch: mem_fetch::MemFetch,
    //     time: usize,
    //     // events: Option<&mut Vec<cache::Event>>,
    //     // events: &[cache::Event],
    //     probe_status: cache::RequestStatus,
    // ) -> cache::RequestStatus {
    //     dbg!((&self.miss_queue.len(), &self.cache_config.miss_queue_size));
    //     dbg!(&self.miss_queue_can_fit(1));
    //     if !self.miss_queue_can_fit(1) {
    //         // cannot handle request this cycle
    //         // (might need to generate two requests)
    //         // m_stats.inc_fail_stats(mf->get_access_type(), MISS_QUEUE_FULL);
    //         return cache::RequestStatus::RESERVATION_FAIL;
    //     }
    //
    //     let block_addr = self.cache_config.block_addr(addr);
    //     let (should_miss, writeback, evicted) = self.send_read_request(
    //         addr,
    //         block_addr,
    //         cache_index,
    //         fetch.clone(),
    //         time,
    //         // events.as_mut().cloned(),
    //         false,
    //         false,
    //     );
    //     dbg!((&should_miss, &writeback, &evicted));
    //
    //     if should_miss {
    //         // If evicted block is modified and not a write-through
    //         // (already modified lower level)
    //         if writeback
    //             && self.cache_config.write_policy != config::CacheWritePolicy::WRITE_THROUGH
    //         {
    //             if let Some(evicted) = evicted {
    //                 let wr = true;
    //                 let access = mem_fetch::MemAccess::new(
    //                     self.write_back_type,
    //                     evicted.block_addr,
    //                     evicted.modified_size as u32,
    //                     wr,
    //                     *fetch.access_warp_mask(),
    //                     evicted.byte_mask,
    //                     evicted.sector_mask,
    //                 );
    //
    //                 // (access, NULL, wr ? WRITE_PACKET_SIZE : READ_PACKET_SIZE, -1,
    //                 //   m_core_id, m_cluster_id, m_memory_config, cycle);
    //                 let mut writeback_fetch = mem_fetch::MemFetch::new(
    //                     fetch.instr,
    //                     access,
    //                     &*self.config,
    //                     if wr {
    //                         ported::WRITE_PACKET_SIZE
    //                     } else {
    //                         ported::READ_PACKET_SIZE
    //                     }
    //                     .into(),
    //                     0,
    //                     0,
    //                     0,
    //                 );
    //
    //                 //     None,
    //                 //     access,
    //                 //     // self.write_back_type,
    //                 //     &*self.config.l1_cache.unwrap(),
    //                 //     // evicted.block_addr,
    //                 //     // evicted.modified_size,
    //                 //     // true,
    //                 //     // fetch.access_warp_mask(),
    //                 //     // evicted.byte_mask,
    //                 //     // evicted.sector_mask,
    //                 //     // m_gpu->gpu_tot_sim_cycle + m_gpu->gpu_sim_cycle,
    //                 //     // -1, -1, -1, NULL,
    //                 // );
    //                 // the evicted block may have wrong chip id when
    //                 // advanced L2 hashing is used, so set the right chip
    //                 // address from the original mf
    //                 writeback_fetch.tlx_addr.chip = fetch.tlx_addr.chip;
    //                 writeback_fetch.tlx_addr.sub_partition = fetch.tlx_addr.sub_partition;
    //                 let event = cache::Event {
    //                     kind: cache::EventKind::WRITE_BACK_REQUEST_SENT,
    //                     evicted_block: None,
    //                 };
    //
    //                 self.send_write_request(
    //                     writeback_fetch,
    //                     event,
    //                     time,
    //                     // &events,
    //                 );
    //             }
    //         }
    //         return cache::RequestStatus::MISS;
    //     }
    //
    //     return cache::RequestStatus::RESERVATION_FAIL;
    // }
}

impl<I> cache::Component for Base<I>
where
    // I: ic::MemPort,
    I: ic::MemFetchInterface,
{
    /// Sends next request to lower level of memory
    fn cycle(&mut self) {
        println!("base cache: cycle");
        dbg!(&self.miss_queue.len());
        if let Some(fetch) = self.miss_queue.front() {
            dbg!(&fetch);
            if !self.mem_port.full(fetch.data_size, fetch.is_write()) {
                if let Some(fetch) = self.miss_queue.pop_front() {
                    self.mem_port.push(fetch);
                }
            }
        }
        let data_port_busy = !self.bandwidth.data_port_free();
        let fill_port_busy = !self.bandwidth.fill_port_free();
        // m_stats.sample_cache_port_utility(data_port_busy, fill_port_busy);
        self.bandwidth.replenish_port_bandwidth();
    }
}

// stop: we do not want to implement cache for base as
// it should not actually implement an access function
// impl<I> cache::Cache for Base<I>
impl<I> Base<I>
where
    // I: ic::MemPort,
    I: ic::MemFetchInterface,
{
    /// Interface for response from lower memory level.
    ///
    /// bandwidth restictions should be modeled in the caller.
    pub fn fill(&self, fetch: &mem_fetch::MemFetch) {
        if self.cache_config.mshr_kind == config::MshrKind::SECTOR_ASSOC {
            // debug_assert!(fetch.get_original_mf());
            // extra_mf_fields_lookup::iterator e =
            //     m_extra_mf_fields.find(mf->get_original_mf());
            // assert(e != m_extra_mf_fields.end());
            // e->second.pending_read--;
            //
            // if (e->second.pending_read > 0) {
            //   // wait for the other requests to come back
            //   delete mf;
            //   return;
            // } else {
            //   mem_fetch *temp = mf;
            //   mf = mf->get_original_mf();
            //   delete temp;
            // }
        }
        // extra_mf_fields_lookup::iterator e = m_extra_mf_fields.find(mf);
        //   assert(e != m_extra_mf_fields.end());
        //   assert(e->second.m_valid);
        //   mf->set_data_size(e->second.m_data_size);
        //   mf->set_addr(e->second.m_addr);
        //   if (m_config.m_alloc_policy == ON_MISS)
        //     m_tag_array->fill(e->second.m_cache_index, time, mf);
        //   else if (m_config.m_alloc_policy == ON_FILL) {
        //     m_tag_array->fill(e->second.m_block_addr, time, mf, mf->is_write());
        //   } else
        //     abort();
        //   bool has_atomic = false;
        //   m_mshrs.mark_ready(e->second.m_block_addr, has_atomic);
        //   if (has_atomic) {
        //     assert(m_config.m_alloc_policy == ON_MISS);
        //     cache_block_t *block = m_tag_array->get_block(e->second.m_cache_index);
        //     if (!block->is_modified_line()) {
        //       m_tag_array->inc_dirty();
        //     }
        //     block->set_status(MODIFIED,
        //                       mf->get_access_sector_mask());  // mark line as dirty for
        //                                                       // atomic operation
        //     block->set_byte_mask(mf);
        //   }
        //   m_extra_mf_fields.erase(mf);
        //   m_bandwidth_management.use_fill_port(mf);
        todo!("l1 base: fill");
    }
}

// impl<I> cache::CacheBandwidth for Base<I> {
//     fn has_free_data_port(&self) -> bool {
//         self.bandwidth_management.has_free_data_port()
//     }
//
//     fn has_free_fill_port(&self) -> bool {
//         self.bandwidth_management.has_free_fill_port()
//     }
// }

#[cfg(test)]
mod tests {
    use super::Base;
    use crate::config;
    use crate::ported::{interconn as ic, mem_fetch, stats::Stats};
    use std::sync::{Arc, Mutex};

    // struct Interconnect {}
    //
    // impl mem_fetch:: for Interconnect {
    //     fn full(&self, size: u32, write: bool) -> bool {
    //         false
    //     }
    //     fn push(&self, mf: mem_fetch::MemFetch) {}
    // }

    #[test]
    fn base_cache_init() {
        let core_id = 0;
        let cluster_id = 0;
        let stats = Arc::new(Mutex::new(Stats::default()));
        let config = Arc::new(config::GPUConfig::default());
        let cache_config = config.data_cache_l1.clone().unwrap();
        // let port = ic::Interconnect {};
        let port = Arc::new(ic::CoreMemoryInterface {});

        let base = Base::new(core_id, cluster_id, port, stats, config, cache_config);
        dbg!(&base);
        assert!(false);
    }
}
