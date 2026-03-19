// ---------- TalosRunner (embedded Python) ----------
#include <pybind11/pybind11.h>
#include <pybind11/embed.h>        // embedded interpreter
#include <pybind11/stl.h>
#include <iostream>
#include <filesystem>
#include "vdf.hpp"
#include "pot_consensus.hpp"
#include "omni_bridge.hpp"
#include "riscv_interface.hpp"
#include "db_backend.hpp"

namespace py = pybind11;

class TalosRunner {
public:
    TalosRunner() {
        py::gil_scoped_acquire acquire;
        py::module_ sys  = py::module_::import("sys");
        py::list  path   = sys.attr("path");

        // force site-packages
        py::module_ site = py::module_::import("site");
        for (auto p : site.attr("getsitepackages")())
            path.attr("insert")(0, p);

        try {
            py::module_ talos_mod = py::module_::import("talos.agent");
            agent = talos_mod.attr("Agent")();
        } catch (const py::error_already_set& e) {
            std::cerr << "talos import failed: " << e.what() << "\npath="
                      << py::str(sys.attr("path")).cast<std::string>() << std::endl;
            throw std::runtime_error("talos not found");
        }
    }
    void start() { py::gil_scoped_acquire acquire; agent.attr("start")(); }
    void stop()  { py::gil_scoped_acquire acquire; agent.attr("stop")();  }
    std::string status() const { py::gil_scoped_acquire acquire; return py::str(agent.attr("status")()); }
private:
    py::object agent;
};

PYBIND11_MODULE(_tal, m) {
    m.doc() = "TAL super-build (Timechain + LunCoSim + talos)";
    py::class_<TalosRunner>(m, "TalosRunner")
        .def(py::init<>())
        .def("start",  &TalosRunner::start)
        .def("stop",   &TalosRunner::stop)
        .def("status", &TalosRunner::status);

    // Timechain bindings
    py::class_<timechain::VDF>(m, "VDF")
        .def(py::init<int>())
        .def("solve", &timechain::VDF::solve)
        .def("verify", &timechain::VDF::verify);

    py::class_<timechain::PoTConsensus>(m, "PoTConsensus")
        .def(py::init<>())
        .def("set_vdf", &timechain::PoTConsensus::set_vdf)
        .def("validate_block", &timechain::PoTConsensus::validate_block);

    py::class_<timechain::OmniBridge>(m, "OmniBridge")
        .def(py::init<>())
        .def("bridge_assets", &timechain::OmniBridge::bridge_assets);

    // RISC-V interface bindings
    py::class_<tal::RISCV_ZKVM>(m, "RISCV_ZKVM")
        .def(py::init<>())
        .def("compile_llvm", &tal::RISCV_ZKVM::compile_llvm)
        .def("execute", &tal::RISCV_ZKVM::execute)
        .def("prove_execution", &tal::RISCV_ZKVM::prove_execution);

    // Database backend bindings
    py::class_<tal::ZKProofDB>(m, "ZKProofDB")
        .def(py::init<const std::string&>())
        .def("store_proof", &tal::ZKProofDB::store_proof)
        .def("get_proof", &tal::ZKProofDB::get_proof);
}

extern "C" __attribute__((visibility("default"))) PyObject* PyInit__tal(void);
