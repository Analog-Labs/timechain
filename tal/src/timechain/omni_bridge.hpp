#pragma once
#include <string>
#include <vector>
#include <cstdint>

namespace timechain {

class OmniBridge {
public:
    OmniBridge() = default;
    void bridge_assets(const std::string& asset_id, double amount);
};

} // namespace timechain
