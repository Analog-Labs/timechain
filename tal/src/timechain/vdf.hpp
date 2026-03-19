#pragma once
#include <string>
#include <vector>

namespace timechain {

class VDF {
public:
    VDF(int difficulty);
    std::vector<uint8_t> solve(const std::vector<uint8_t>& seed);
    bool verify(const std::vector<uint8_t>& seed, const std::vector<uint8_t>& proof);

private:
    int difficulty_;
};

} // namespace timechain
