# vrf_beacon.py
import os
import time
import hashlib
import base64
import pyvrf

# -------------------------------------------------
# 1️⃣  Generate or load a keypair
# -------------------------------------------------
def load_or_create_keys(path="vrf_key.pem"):
    """Loads a private key from path or generates a new one."""
    if os.path.exists(path):
        with open(path, "rb") as f:
            private_key = f.read()
            # We need to derive the public key from the private key
            public_key = pyvrf.crypto_vrf_sk_to_pk(private_key)

    else:
        public_key, private_key = pyvrf.crypto_vrf_keypair()
        with open(path, "wb") as f:
            f.write(private_key)

    return private_key, public_key

# -------------------------------------------------
# 2️⃣  Beacon source (simple time‑based beacon)
# -------------------------------------------------
def current_beacon(interval_sec=30) -> bytes:
    """Round current unix time to nearest `interval_sec` and return as bytes."""
    t = int(time.time() // interval_sec * interval_sec)
    return t.to_bytes(8, "big")   # 8‑byte big‑endian timestamp

# -------------------------------------------------
# 3️⃣  VRF proof & output
# -------------------------------------------------
def generate_vrf_proof(sk: bytes, message: bytes) -> bytes:
    """Generates a VRF proof."""
    return pyvrf.crypto_vrf_prove(sk, message)

def get_vrf_output(pk: bytes, proof: bytes, message: bytes) -> bytes:
    """Verifies a VRF proof and returns the output hash."""
    return pyvrf.crypto_vrf_verify(pk, proof, message)

# -------------------------------------------------
# 4️⃣  Seed derivation
# -------------------------------------------------
def derive_seed(vrf_out: bytes, beacon: bytes, hash_alg="sha256") -> bytes:
    h = hashlib.new(hash_alg)
    h.update(vrf_out + beacon)
    return h.digest()

# -------------------------------------------------
# 5️⃣  End‑to‑end demo
# -------------------------------------------------
if __name__ == "__main__":
    sk, pk = load_or_create_keys()
    beacon = current_beacon()

    # The VRF message can be any domain‑separator + epoch ID; we use the beacon itself.
    proof = generate_vrf_proof(sk, beacon)

    # Verifiers can independently verify the proof and get the same output.
    try:
        vrf_out = get_vrf_output(pk, proof, beacon)
        print("VRF proof verified successfully!")
    except ValueError as e:
        print(f"VRF proof failed verification: {e}")
        exit(1)

    seed = derive_seed(vrf_out, beacon)

    print("Beacon :", int.from_bytes(beacon, "big"))
    print("VRF out:", base64.b64encode(vrf_out).decode())
    print("Proof  :", base64.b64encode(proof).decode())
    print("Seed   :", seed.hex())
