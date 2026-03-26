#include <pybind11/pybind11.h>
#include "lunco_sim.hpp"   // existing LunCoSim header

namespace py = pybind11;

PYBIND11_MODULE(lunco, m) {
    m.doc() = "Thin Python face for LunCoSim";

    py::class_<LunCoSim::Session>(m, "Session")
        .def(py::init<>())
        .def("load_scenario", &LunCoSim::Session::load_scenario)
        .def("step",          &LunCoSim::Session::step)
        .def("telemetry",     &LunCoSim::Session::get_telemetry);
}
