#pragma once
#include "vdf.hpp"
#include <memory>

namespace timechain {

class PoTConsensus {
public:
    PoTConsensus();
    void set_vdf(std::shared_ptr<VDF> vdf);
    bool validate_block(const std::vector<uint8_t>& block_data, const std::vector<uint8_t>& pot_proof);

private:
    std::shared_ptr<VDF> vdf_;
};

} // namespace timechain
