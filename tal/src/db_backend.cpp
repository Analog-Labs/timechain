#include "db_backend.hpp"

namespace tal {

ZKProofDB::ZKProofDB(const std::string& db_path) : path_(db_path) {}

void ZKProofDB::store_proof(const std::string& execution_id, const std::vector<uint8_t>& proof) {
    // Skeleton for storing ZK proofs in a database backend
    mock_storage_[execution_id] = proof;
}

std::vector<uint8_t> ZKProofDB::get_proof(const std::string& execution_id) {
    // Skeleton for retrieving ZK proofs from a database backend
    if (mock_storage_.find(execution_id) != mock_storage_.end()) {
        return mock_storage_[execution_id];
    }
    return {};
}

} // namespace tal
