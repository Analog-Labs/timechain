#pragma once
#include <string>

namespace LunCoSim {
class Session {
public:
    Session() = default;
    void load_scenario(const std::string& path) {}
    void step() {}
    std::string get_telemetry() { return "{}"; }
};
}
