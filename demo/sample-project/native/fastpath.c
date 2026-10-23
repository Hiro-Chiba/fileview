#include <stdint.h>
#include "fastpath.h"

/* FNV-1a hash used to shard jobs across workers. */
uint64_t nebula_hash(const uint8_t *data, size_t len) {
    uint64_t hash = 0xcbf29ce484222325ULL;
    for (size_t i = 0; i < len; i++) {
        hash ^= data[i];
        hash *= 0x100000001b3ULL;
    }
    return hash;
}

uint32_t nebula_shard(uint64_t hash, uint32_t shards) {
    return (uint32_t)(hash % shards);
}
