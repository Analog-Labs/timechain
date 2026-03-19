# arkhetv/poc-ai/poc_ai.py
import hashlib
import time
import os
try:
    import tal
except ImportError:
    tal = None

class HistoryBackend:
    """
    ZK-proven history storage and database backend.
    Stores and retrieves interaction traces validated against the ZK-proof database.
    """
    def __init__(self, db_path="arkhe_history.db"):
        self.db = tal.ZKProofDB(db_path) if tal and hasattr(tal, 'ZKProofDB') else None
        self.local_storage = {}

    def store_interaction(self, user_id, action, proof=None):
        """Stores a user interaction and its corresponding ZK-proof."""
        timestamp = time.time()
        interaction_id = hashlib.sha256(f"{user_id}_{action}_{timestamp}".encode()).hexdigest()

        print(f"[BACKEND] Storing interaction: {interaction_id[:16]}...")

        if self.db and proof:
            self.db.store_proof(interaction_id, proof)

        self.local_storage[interaction_id] = {
            'user_id': user_id,
            'action': action,
            'timestamp': timestamp,
            'has_proof': proof is not None
        }
        return interaction_id

    def get_interaction(self, interaction_id):
        """Retrieves an interaction and its ZK-proof."""
        interaction = self.local_storage.get(interaction_id)
        if not interaction:
            return None

        if self.db and interaction['has_proof']:
            proof = self.db.get_proof(interaction_id)
            interaction['proof'] = proof

        return interaction

    def get_user_history(self, user_id):
        """Retrieves all interactions for a specific user."""
        return [i for i in self.local_storage.values() if i['user_id'] == user_id]

if __name__ == "__main__":
    backend = HistoryBackend()
    inter_id = backend.store_interaction("user_001", "WATCH_AMAZONIA", b"MOCK_PROOF")
    print(f"Stored: {backend.get_interaction(inter_id)}")
