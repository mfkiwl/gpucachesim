#include "shader_core_mem_fetch_allocator.hpp"

mem_fetch *shader_core_mem_fetch_allocator::alloc(
    new_addr_type addr, mem_access_type type, unsigned size, bool wr,
    unsigned long long cycle) const {
  mem_access_t access(type, addr, size, wr, m_memory_config->gpgpu_ctx);
  mem_fetch *mf =
      new mem_fetch(access, NULL, wr ? WRITE_PACKET_SIZE : READ_PACKET_SIZE, -1,
                    m_core_id, m_cluster_id, m_memory_config, cycle);
  return mf;
}

mem_fetch *shader_core_mem_fetch_allocator::alloc(
    new_addr_type addr, mem_access_type type, const active_mask_t &active_mask,
    const mem_access_byte_mask_t &byte_mask,
    const mem_access_sector_mask_t &sector_mask, unsigned size, bool wr,
    unsigned long long cycle, unsigned wid, unsigned sid, unsigned tpc,
    mem_fetch *original_mf) const {
  mem_access_t access(type, addr, size, wr, active_mask, byte_mask, sector_mask,
                      m_memory_config->gpgpu_ctx);
  mem_fetch *mf = new mem_fetch(
      access, NULL, wr ? WRITE_PACKET_SIZE : READ_PACKET_SIZE, wid, m_core_id,
      m_cluster_id, m_memory_config, cycle, original_mf);
  return mf;
}
