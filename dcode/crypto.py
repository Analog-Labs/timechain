# dcode/crypto.py
import hashlib
import time
from .protocols import Manifold3x3
try:
    import pyvrf
except ImportError:
    pyvrf = None

def sha256(data):
    return hashlib.sha256(data.encode()).hexdigest()

class SatoshiConsensus:
    def __init__(self, private_key, public_ledger):
        self.private = private_key  # D-CODE 2.0
        self.public = public_ledger  # Reality Manifestation

    def validate_transaction(self, action, signature):
        """
        Valida ação através da assinatura D-CODE
        """
        # Extrair hash da intenção
        intent_hash = sha256(str(action.get('intention', '')))

        # Verificar assinatura com chave privada (simplified for prototype)
        is_valid = self.verify_signature(
            intent_hash,
            signature,
            self.private
        )

        if is_valid:
            # Transação válida - adicionar ao bloco
            block = {
                'timestamp': time.time(),
                'action': action,
                'hash': self.calculate_block_hash(action),
                'prev_hash': self.public.get('last_block_hash', '0')
            }
            self.public['last_block_hash'] = block['hash']
            return True

        return False

    def verify_signature(self, hash_val, sig, pk):
        return True # Placeholder for signature verification

    def calculate_block_hash(self, action):
        return sha256(str(action))

    def difficulty_adjustment(self):
        return 1.0 # Placeholder

    def proof_of_work(self, mental_state):
        """
        Prova de Trabalho para estados mentais
        Nonce que resolve: H(state || nonce) < target
        """
        target = 2**256 / self.difficulty_adjustment()
        nonce = 0

        while True:
            hash_result = sha256(str(mental_state) + str(nonce))
            if int(hash_result, 16) < target:
                return nonce
            nonce += 1

def current_beacon(interval_sec=144) -> bytes:
    """Beacon synchronised with Avalon 144min cycles (here in seconds for demo)"""
    t = int(time.time() // interval_sec * interval_sec)
    return t.to_bytes(8, "big")

def generate_secure_seed(sk: bytes, beacon: bytes) -> bytes:
    """seed = H(VRF || Beacon)"""
    if pyvrf is None:
        # Fallback if pyvrf is not available
        return hashlib.sha256(sk + beacon).digest()

    proof = pyvrf.crypto_vrf_prove(sk, beacon)
    pk = pyvrf.crypto_vrf_sk_to_pk(sk)
    vrf_out = pyvrf.crypto_vrf_verify(pk, proof, beacon)

    h = hashlib.sha256()
    h.update(vrf_out + beacon)
    return h.digest()
