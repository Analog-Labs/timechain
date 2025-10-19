// ---------- TalosRunner (embedded Python) ----------
#include <pybind11/pybind11.h>
#include <pybind11/embed.h>        // embedded interpreter
#include <pybind11/stl.h>
#include <iostream>
#include <filesystem>

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
    // existing Timechain/LunCoSim bindings stay here
}

extern "C" __attribute__((visibility("default"))) PyObject* PyInit__tal(void);
