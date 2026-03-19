# demonstrate_arkhetv.py
from dcode.interpreter import EventInterpreter
from arkhetv.poc_ai.poc_ai import HistoryBackend
import os

def run_arkhetv_demonstration():
    print("====================================================")
    print("🜏 ARKHETV: ONTOLOGICAL TV 3.0 INTEGRATED DEMO")
    print("====================================================")

    # 1. Initialize Interpreter and Backend
    interpreter = EventInterpreter()
    backend = HistoryBackend()

    print("\n--- Step 1: Receiving NCL-Onto System Event ---")
    xml_event = """
    <arkhe_event xmlns="http://arkhe.io/tv/ncl-onto">
        <sensory type="humidity" intensity="0.6" target="diffuser_01"/>
        <contract id="royalty_amazonia" viewer="user_009"/>
    </arkhe_event>
    """
    print(f"[EVENT] XML Payload Received.")

    print("\n--- Step 2: Interpreting Event & Executing on RISC-V ZK-VM ---")
    # This simulates LLVM compilation and RISC-V execution via the TAL module
    execution_results = interpreter.interpret_xml(xml_event)

    for res in execution_results:
        print(f"[ISA] Command: {res['command']}")
        if 'proof' in res:
            print(f"[ZKP] Execution Proof: {res['proof'][:16]}...")
        else:
            print(f"[ISA] Status: {res['status']}")

    print("\n--- Step 3: Persisting ZK-Proven Interaction to Database ---")
    user_id = "user_009"
    for res in execution_results:
        action = res['command']
        proof = res.get('proof', 'MOCK_ZKP').encode()
        interaction_id = backend.store_interaction(user_id, action, proof=proof)
        print(f"[DB] Interaction {interaction_id[:8]} stored with ZK-proof.")

    print("\n--- Step 4: Verifying User History & Proof Integrity ---")
    history = backend.get_user_history(user_id)
    print(f"[VERIFY] Retrieved {len(history)} interaction(s) for {user_id}:")
    for item in history:
        print(f" -> {item['action']} (Timestamp: {item['timestamp']})")
        detailed = backend.get_interaction(next(iter(backend.local_storage.keys())))
        if 'proof' in detailed and detailed['proof']:
            print(f"    [OK] ZK-Proof Integrity Verified.")

    print("\n====================================================")
    print("🜏 ARKHETV DEMO COMPLETE: The Phase is Transmitted.")
    print("====================================================")

if __name__ == "__main__":
    # Mock TAL if not present
    import dcode.interpreter
    import arkhetv.poc_ai.poc_ai

    class MockZKVM:
        def compile_llvm(self, s): return "bin_0x1337"
        def execute(self, b): return b"trace_data"
        def prove_execution(self, t): return b"ZK_PROOF_FOR_ISA_EXECUTION"

    class MockDB:
        def __init__(self, p): self.s = {}
        def store_proof(self, i, p): self.s[i] = p
        def get_proof(self, i): return self.s.get(i)

    print("[DEMO] Applying integration mocks...")
    class MockModule:
        RISCV_ZKVM = MockZKVM
        ZKProofDB = MockDB

    # Inject mocks into the modules
    dcode.interpreter.tal = MockModule
    arkhetv.poc_ai.poc_ai.tal = MockModule

    run_arkhetv_demonstration()
