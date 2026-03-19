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

class EntropyShield:
    """
    Entropy Shield & Thermal Firewall
    Protects against 'False Past Injections' by monitoring signature entropy.
    """
    def __init__(self, threshold=0.001):
        self.threshold = threshold
        self.absorbed_entropy = 0.0

    def validate_phase_signature(self, signature_data):
        """
        Calculates Shannon entropy of the phase signature.
        H(Φ) = -Σ p(Φᵢ) log p(Φᵢ)
        """
        import math
        if not signature_data: return True

        # Calculate frequencies
        counts = {}
        for b in signature_data:
            counts[b] = counts.get(b, 0) + 1

        probs = [c / len(signature_data) for c in counts.values()]
        entropy = -sum(p * math.log2(p) for p in probs)

        # False past injections often have ANOMALOUS entropy (too low or artificial)
        # For this demo, we simulate a 'thermal breach' if entropy is outside expected range
        if entropy < 1.0: # Artificial/forged signatures have low entropy
            return self.activate_thermal_firewall(entropy)

        return True

    def activate_thermal_firewall(self, anomalous_entropy):
        """
        Absorbs excess entropy and dissipates it as simulated π+ pions.
        """
        energy_to_dissipate = (1.0 - anomalous_entropy) * 2.4 # J/s scaling
        self.absorbed_entropy += energy_to_dissipate
        print(f"[SHIELD] !!! Thermal Breach Detected (Entropy: {anomalous_entropy:.4f}) !!!")
        print(f"[SHIELD] Firewall active: Dissipating {energy_to_dissipate:.2f} J/s as simulated π+ pions.")
        return False

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
