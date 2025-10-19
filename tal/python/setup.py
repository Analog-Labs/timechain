from setuptools import setup, Extension
from pybind11.setup_helpers import Pybind11Extension, build_ext

def talos_site():
    import site, os
    try:
        # venv
        return next(p for p in site.getsitepackages() if 'site-packages' in p)
    except StopIteration:
        # system
        return site.getsitepackages()[0]

ext_modules = [
    Pybind11Extension(
        "tal._tal",
        ["tal.cpp"],
        include_dirs=["../src"],
        cxx_std=20,
        define_macros=[("PY_TALOS_SITE_PACKAGES", '"' + talos_site() + '"')],
    ),
]

setup(
    name="tal",
    ext_modules=ext_modules,
    cmdclass={"build_ext": build_ext},
    install_requires=["numpy>=1.24"],
    packages=["tal"],
)
