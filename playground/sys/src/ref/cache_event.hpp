#pragma once

#include "cache_event_type.hpp"
#include "evicted_block_info.hpp"

struct cache_event {
  enum cache_event_type m_cache_event_type;
  evicted_block_info m_evicted_block;  // if it was write_back event, fill the
                                       // the evicted block info

  cache_event(enum cache_event_type m_cache_event) {
    m_cache_event_type = m_cache_event;
  }

  cache_event(enum cache_event_type cache_event,
              evicted_block_info evicted_block) {
    m_cache_event_type = cache_event;
    m_evicted_block = evicted_block;
  }
};

#include "fmt/core.h"

template <>
struct fmt::formatter<cache_event> {
  constexpr auto parse(format_parse_context &ctx)
      -> format_parse_context::iterator {
    return ctx.end();
  }

  auto format(const cache_event &event, format_context &ctx) const
      -> format_context::iterator {
    return fmt::format_to(ctx.out(), "{}(evicted={})",
                          cache_event_type_str[event.m_cache_event_type],
                          event.m_evicted_block);
  }
};
