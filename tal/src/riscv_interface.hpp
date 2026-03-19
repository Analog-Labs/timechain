#pragma once
#include <string>
#include <vector>

namespace tal {

class RISCV_ZKVM {
public:
    RISCV_ZKVM();
    std::string compile_llvm(const std::string& source_code);
    std::vector<uint8_t> execute(const std::string& binary);
    std::vector<uint8_t> prove_execution(const std::vector<uint8_t>& trace);
};

} // namespace tal
