# dcode/physics.py
import numpy as np

def phi_c(s, t, geometry_func):
    """Campo de restrição geométrica."""
    return geometry_func(s, t)

def is_admissible(s, t, geometry_func):
    """Estado admissível: S ∈ Adm ⇔ Φ_C(S, t) > 0"""
    return phi_c(s, t, geometry_func) > 0

def theta(x):
    """Função degrau."""
    return 1.0 if x > 0 else 0.0

def state_transition_rate(s, t, geometry_func, grad_phi_c):
    """
    Transição: dS/dt = -∇Φ_C(S, t) · Θ(Φ_C(S, t))
    """
    val = phi_c(s, t, geometry_func)
    return -grad_phi_c(s, t) * theta(val)

def gamma_t(t, tau_d, sigma_n, v):
    """
    Γ(t) = exp(-t/τ_d)·[1 - exp(-⟨σ·n⟩·v·t)]
    """
    return np.exp(-t/tau_d) * (1 - np.exp(-sigma_n * v * t))

def energy_released(t_start, t_end, e_metastable, e_fundamental, tau_d, sigma_n, v):
    """
    ΔE_liberado = ∫[E_metaestável - E_fundamental]·Γ(t) dt
    Simplified as a numerical integration placeholder.
    """
    dt = 0.1
    t_range = np.arange(t_start, t_end, dt)
    integrand = (e_metastable - e_fundamental) * gamma_t(t_range, tau_d, sigma_n, v)
    return np.sum(integrand) * dt

def critical_point(hess_phi_c_func, states, times):
    """
    t_critical = min{t | det(Hess(Φ_C)(S, t)) = 0}
    """
    for t in times:
        for s in states:
            hess = hess_phi_c_func(s, t)
            if np.isclose(np.linalg.det(hess), 0, atol=1e-5):
                return t
    return None
