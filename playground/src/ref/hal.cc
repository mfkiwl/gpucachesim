#include "hal.hpp"

const char *uarch_op_t_str[]{"NO_OP",
                             "ALU_OP",
                             "SFU_OP",
                             "TENSOR_CORE_OP",
                             "DP_OP",
                             "SP_OP",
                             "INTP_OP",
                             "ALU_SFU_OP",
                             "LOAD_OP",
                             "TENSOR_CORE_LOAD_OP",
                             "TENSOR_CORE_STORE_OP",
                             "STORE_OP",
                             "BRANCH_OP",
                             "BARRIER_OP",
                             "MEMORY_BARRIER_OP",
                             "CALL_OPS",
                             "RET_OPS",
                             "EXIT_OPS",
                             "SPECIALIZED_UNIT_1_OP",
                             "SPECIALIZED_UNIT_2_OP",
                             "SPECIALIZED_UNIT_3_OP",
                             "SPECIALIZED_UNIT_4_OP",
                             "SPECIALIZED_UNIT_5_OP",
                             "SPECIALIZED_UNIT_6_OP",
                             "SPECIALIZED_UNIT_7_OP",
                             "SPECIALIZED_UNIT_8_OP"};

const char *str_to_bool(bool value) { return (value) ? "true" : "false"; }
