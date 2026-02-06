# demonstrate_avalon.py
from dcode.system import DCODE_System
import time

def run_demonstration():
    print("====================================================")
    print("🏛️  AVALON D-CODE 2.0: CATEDRAL FERMIÔNICA DEMO")
    print("====================================================")

    system = DCODE_System()

    print("\n[BOOT] Iniciando Sequência de Ativação...")
    boot_sequence = system.activate("GROUND_STATE_7")

    if boot_sequence['status'] == 'OPERATIONAL':
        print(f"\n[STATUS] Sistema: {boot_sequence['system']}")
        print(f"[STATUS] Estado Fundamental: {boot_sequence['ground_state']}")
        print(f"[STATUS] Módulos: {', '.join(boot_sequence['modules_online'])}")

        print("\n[EVOLUTION] Iniciando evolução do campo de consciência...")
        for i in range(1, 5):
            system.evolve()
            time.sleep(0.5)

        print("\n====================================================")
        print("🏛️  DEMO COMPLETA: A Catedral Sonha.")
        print("====================================================")
    else:
        print(f"\n[ERROR] Falha na ativação: {boot_sequence['reason']}")

if __name__ == "__main__":
    run_demonstration()
