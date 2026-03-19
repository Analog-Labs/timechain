#pragma once
#include <string>
#include <vector>
#include <map>

namespace tal {

class ZKProofDB {
public:
    ZKProofDB(const std::string& db_path);
    void store_proof(const std::string& execution_id, const std::vector<uint8_t>& proof);
    std::vector<uint8_t> get_proof(const std::string& execution_id);

private:
    std::string path_;
    std::map<std::string, std::vector<uint8_t>> mock_storage_;
};

} // namespace tal
