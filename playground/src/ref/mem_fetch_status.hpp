#pragma once

static const char *Status_str[] = {
    "MEM_FETCH_INITIALIZED",
    "IN_L1I_MISS_QUEUE",
    "IN_L1D_MISS_QUEUE",
    "IN_L1T_MISS_QUEUE",
    "IN_L1C_MISS_QUEUE",
    "IN_L1TLB_MISS_QUEUE",
    "IN_VM_MANAGER_QUEUE",
    "IN_ICNT_TO_MEM",
    "IN_PARTITION_ROP_DELAY",
    "IN_PARTITION_ICNT_TO_L2_QUEUE",
    "IN_PARTITION_L2_TO_DRAM_QUEUE",
    "IN_PARTITION_DRAM_LATENCY_QUEUE",
    "IN_PARTITION_L2_MISS_QUEUE",
    "IN_PARTITION_MC_INTERFACE_QUEUE",
    "IN_PARTITION_MC_INPUT_QUEUE",
    "IN_PARTITION_MC_BANK_ARB_QUEUE",
    "IN_PARTITION_DRAM",
    "IN_PARTITION_MC_RETURNQ",
    "IN_PARTITION_DRAM_TO_L2_QUEUE",
    "IN_PARTITION_L2_FILL_QUEUE",
    "IN_PARTITION_L2_TO_ICNT_QUEUE",
    "IN_ICNT_TO_SHADER",
    "IN_CLUSTER_TO_SHADER_QUEUE",
    "IN_SHADER_LDST_RESPONSE_FIFO",
    "IN_SHADER_FETCHED",
    "IN_SHADER_L1T_ROB",
    "MEM_FETCH_DELETED",
    "NUM_MEM_REQ_STAT",
};

enum mem_fetch_status {
  MEM_FETCH_INITIALIZED,
  IN_L1I_MISS_QUEUE,
  IN_L1D_MISS_QUEUE,
  IN_L1T_MISS_QUEUE,
  IN_L1C_MISS_QUEUE,
  IN_L1TLB_MISS_QUEUE,
  IN_VM_MANAGER_QUEUE,
  IN_ICNT_TO_MEM,
  IN_PARTITION_ROP_DELAY,
  IN_PARTITION_ICNT_TO_L2_QUEUE,
  IN_PARTITION_L2_TO_DRAM_QUEUE,
  IN_PARTITION_DRAM_LATENCY_QUEUE,
  IN_PARTITION_L2_MISS_QUEUE,
  IN_PARTITION_MC_INTERFACE_QUEUE,
  IN_PARTITION_MC_INPUT_QUEUE,
  IN_PARTITION_MC_BANK_ARB_QUEUE,
  IN_PARTITION_DRAM,
  IN_PARTITION_MC_RETURNQ,
  IN_PARTITION_DRAM_TO_L2_QUEUE,
  IN_PARTITION_L2_FILL_QUEUE,
  IN_PARTITION_L2_TO_ICNT_QUEUE,
  IN_ICNT_TO_SHADER,
  IN_CLUSTER_TO_SHADER_QUEUE,
  IN_SHADER_LDST_RESPONSE_FIFO,
  IN_SHADER_FETCHED,
  IN_SHADER_L1T_ROB,
  MEM_FETCH_DELETED,
  NUM_MEM_REQ_STAT,
};
