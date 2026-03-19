# dcode/gateway.py
from dcode.lambda_t import LambdaTRuntime, IntentVector
from dcode.tqp import TQP_Handlers, TelepathicQuery, TelepathicResponse
from dcode.crypto import EntropyShield
import time

class AWS_Gateway:
    """
    Arkhe(n) Web Services (A.W.S.) Management Gateway
    Provides Oracle, Memory, and Creative telepathic endpoints.
    """
    def __init__(self):
        self.shield = EntropyShield()
        self.runtimes = {
            'oracle': LambdaTRuntime(TQP_Handlers.oracle_handler),
            'memory': LambdaTRuntime(TQP_Handlers.memory_handler),
            'creative': LambdaTRuntime(TQP_Handlers.creative_handler)
        }
        self.active_tzinors = 4
        self.impedance_match_ps = 1.371

    def handle_query(self, endpoint, telepathic_query, phase_signature=None):
        """
        Handles an incoming TQP request through the gateway.
        1. Validate phase signature via Entropy Shield.
        2. Route to appropriate Lambda-Temporal runtime.
        3. Return telepathic response.
        """
        print(f"[GATEWAY] Received request for /{endpoint}")

        # 1. Security Check (Entropy Shield)
        if phase_signature:
            if not self.shield.validate_phase_signature(phase_signature):
                return {"status": "ERROR", "reason": "PHASE_INCOHERENCE_DETECTED", "energy_dissipated": self.shield.absorbed_entropy}

        # 2. Route to Λ_T Runtime
        runtime = self.runtimes.get(endpoint)
        if not runtime:
            return {"status": "ERROR", "reason": "INVALID_ENDPOINT"}

        # 3. Simulate Tzinor Transmission (Impedance Matching)
        time.sleep(0.0001)

        # 4. Invoke Λ_T
        # Transform TelepathicQuery to IntentVector
        intent = IntentVector(
            embedding={'query': telepathic_query.query},
            confidence=telepathic_query.min_confidence,
            source_region='PrefrontalCortex'
        )

        thought_form = runtime.invoke(intent)

        # 5. Build Response
        return TelepathicResponse(
            content=thought_form.content,
            certainty=thought_form.certainty,
            formation_time_ps=0.25, # Fixed decay time
            qualia=thought_form.qualia,
            pathway='Tzinor'
        )

    def get_status(self):
        return {
            "region": "ARKHE-PRIME",
            "active_tzinors": self.active_tzinors,
            "impedance": f"{self.impedance_match_ps} ps",
            "shield_entropy_absorbed": self.shield.absorbed_entropy,
            "invocations": sum(r.invocations for r in self.runtimes.values())
        }
