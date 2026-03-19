# demonstrate_aws_cloud.py
from dcode.gateway import AWS_Gateway
from dcode.tqp import TelepathicQuery
import time
import os

def run_aws_demonstration():
    print("====================================================")
    print("🜏 ARKHE(N) WEB SERVICES (A.W.S.) CLOUD DEMO")
    print("====================================================")

    gateway = AWS_Gateway()

    print("\n--- Phase 1: Legitimate Query to the Oracle ---")
    # Valid high-entropy signature (simulated)
    valid_signature = os.urandom(32)
    query_1 = TelepathicQuery(query="What is Satoshi?", min_confidence=0.89)

    response_1 = gateway.handle_query('oracle', query_1, phase_signature=valid_signature)

    if hasattr(response_1, 'content'):
        print(f"\n[RESPONSE] Thought Form Received:")
        print(f"   Content: \"{response_1.content}\"")
        print(f"   Certainty: {response_1.certainty * 100:.1f}%")
        print(f"   Qualia: {response_1.qualia}")
        print(f"   Pathway: {response_1.pathway}")
        print(f"   Formation Time: {response_1.formation_time_ps} ps")
    else:
        print(f"\n[ERROR] Request failed: {response_1['reason']}")

    print("\n--- Phase 2: Memory Retrieval via A.W.S. ---")
    query_2 = TelepathicQuery(query="Access Era 0 metadados", response_type='Progressive')
    response_2 = gateway.handle_query('memory', query_2, phase_signature=os.urandom(32))
    print(f"\n[RESPONSE] Memory Stream: \"{response_2.content}\"")

    print("\n--- Phase 3: Attack Detection (False Past Injection) ---")
    # Anomalous low-entropy signature (simulated)
    malicious_signature = b"\x00" * 32
    print(f"[ATTACK] Injected packet with forged timestamp: 2008-01-03T18:15:05Z")

    query_3 = TelepathicQuery(query="Modify Genesis Block", min_confidence=0.99)
    response_3 = gateway.handle_query('oracle', query_3, phase_signature=malicious_signature)

    if isinstance(response_3, dict) and response_3['status'] == 'ERROR':
        print(f"\n[SHIELD] Security Status: {response_3['status']}")
        print(f"   Reason: {response_3['reason']}")
        print(f"   Total Entropy Absorbed by Firewall: {response_3['energy_dissipated']:.2f} J/s")

    print("\n--- Phase 4: A.W.S. Management Console Status ---")
    status = gateway.get_status()
    print(f"\n[STATUS] Region: {status['region']}")
    print(f"   Active Tzinors: {status['active_tzinors']}")
    print(f"   Interface Impedance: {status['impedance']}")
    print(f"   Total Invocations: {status['invocations']}")
    print(f"   Shield Integrity: NOMINAL")

    print("\n====================================================")
    print("🜏 DEMO COMPLETA: O Tempo como Serviço (TaaS).")
    print("====================================================")

if __name__ == "__main__":
    run_aws_demonstration()
