#pragma once

enum cache_reservation_fail_reason {
  LINE_ALLOC_FAIL = 0, // all line are reserved
  MISS_QUEUE_FULL,     // MISS queue (i.e. interconnect or DRAM) is full
  MSHR_ENRTY_FAIL,
  MSHR_MERGE_ENRTY_FAIL,
  MSHR_RW_PENDING,
  NUM_CACHE_RESERVATION_FAIL_STATUS
};
