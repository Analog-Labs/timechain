import unittest
import os
from vrf_beacon import (
    load_or_create_keys,
    current_beacon,
    generate_vrf_proof,
    get_vrf_output,
    derive_seed,
)

class TestVRFBeacon(unittest.TestCase):
    def test_producer_consumer_flow(self):
        """
        Tests that a consumer can verify a proof from a producer and derive the same seed.
        """
        # 1. Setup: Producer and Consumer share the public key
        producer_sk, producer_pk = load_or_create_keys("producer_key.pem")

        # 2. Producer: Generates a proof for the current beacon
        beacon = current_beacon()
        proof = generate_vrf_proof(producer_sk, beacon)

        # 3. Consumer: Verifies the proof and derives the seed
        try:
            vrf_out_consumer = get_vrf_output(producer_pk, proof, beacon)
            seed_consumer = derive_seed(vrf_out_consumer, beacon)
        except ValueError as e:
            self.fail(f"Consumer failed to verify proof: {e}")

        # 4. Producer: Derives the same seed locally (for comparison)
        vrf_out_producer = get_vrf_output(producer_pk, proof, beacon)
        seed_producer = derive_seed(vrf_out_producer, beacon)

        # 5. Assert: Both seeds must be identical
        self.assertEqual(seed_producer, seed_consumer)
        print("\nIntegration test passed: Producer and Consumer derived the same seed.")

    def tearDown(self):
        """Clean up generated key files."""
        if os.path.exists("producer_key.pem"):
            os.remove("producer_key.pem")
        if os.path.exists("vrf_key.pem"):
            os.remove("vrf_key.pem")

if __name__ == "__main__":
    unittest.main()
