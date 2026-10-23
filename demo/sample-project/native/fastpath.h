#ifndef NEBULA_FASTPATH_H
#define NEBULA_FASTPATH_H

#include <stddef.h>
#include <stdint.h>

uint64_t nebula_hash(const uint8_t *data, size_t len);
uint32_t nebula_shard(uint64_t hash, uint32_t shards);

#endif
