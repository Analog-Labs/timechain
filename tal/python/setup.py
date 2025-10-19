from pybind11.setup_helpers import Pybind11Extension, build_ext
from setuptools import setup

ext_modules = [
    Pybind11Extension(
        "tal._tal",
        ["tal.cpp"],
        include_dirs=["../src"],
        cxx_std=20,
    ),
]

setup(
    name="tal",
    ext_modules=ext_modules,
    cmdclass={"build_ext": build_ext},
    packages=["tal"],
    install_requires=["numpy>=1.24"],
)
