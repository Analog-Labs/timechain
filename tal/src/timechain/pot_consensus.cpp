#include "pot_consensus.hpp"

namespace timechain {

PoTConsensus::PoTConsensus() : vdf_(nullptr) {}

void PoTConsensus::set_vdf(std::shared_ptr<VDF> vdf) {
    vdf_ = vdf;
}

bool PoTConsensus::validate_block(const std::vector<uint8_t>& block_data, const std::vector<uint8_t>& pot_proof) {
    if (!vdf_) return false;
    return vdf_->verify(block_data, pot_proof);
}

} // namespace timechain
