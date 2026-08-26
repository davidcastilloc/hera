"""Token usage, cost tracking and visual snapbar for Hera AI Agent."""

from dataclasses import dataclass
from typing import Any

# Precios estimados por 1 Millón de tokens (USD)
# Formato: (input_usd_per_1m, output_usd_per_1m)
MODEL_PRICING_USD: dict[str, tuple[float, float]] = {
    # Gemini / Vertex
    "gemini-2.5-flash": (0.075, 0.30),
    "gemini-2.0-flash": (0.075, 0.30),
    "gemini-1.5-flash": (0.075, 0.30),
    "gemini-1.5-flash-8b": (0.0375, 0.15),
    "gemini-2.5-pro": (1.25, 5.00),
    "gemini-1.5-pro": (1.25, 5.00),
    # OpenAI
    "gpt-4o": (2.50, 10.00),
    "gpt-4o-mini": (0.15, 0.60),
    "o1": (15.00, 60.00),
    "o3-mini": (1.10, 4.40),
    # Anthropic
    "claude-3-5-sonnet": (3.00, 15.00),
    "claude-sonnet-4": (3.00, 15.00),
    "claude-3-5-haiku": (0.80, 4.00),
    "claude-3-opus": (15.00, 75.00),
}


def get_model_rates(model_name: str, is_local: bool = False) -> tuple[float, float]:
    """Obtiene tarifas (input_rate, output_rate) por token para el modelo."""
    if is_local:
        return (0.0, 0.0)

    model_clean = model_name.lower()
    for key, (in_price, out_price) in MODEL_PRICING_USD.items():
        if key in model_clean:
            return (in_price / 1_000_000.0, out_price / 1_000_000.0)

    return (0.10 / 1_000_000.0, 0.40 / 1_000_000.0)


@dataclass
class CostTracker:
    """Rastreador acumulativo de consumo de tokens y costos por sesión de chat."""
    backend_name: str = "auto"
    model_name: str = "gemini-2.5-flash"
    is_local: bool = False
    max_session_cost_usd: float | None = None

    # Métricas acumuladas de la sesión
    total_turns: int = 0
    total_prompt_tokens: int = 0
    total_completion_tokens: int = 0
    total_tokens: int = 0
    total_cost_usd: float = 0.0

    # Métricas del último turno
    last_prompt_tokens: int = 0
    last_completion_tokens: int = 0
    last_tokens: int = 0
    last_cost_usd: float = 0.0

    # Alertas
    budget_warned: bool = False

    def record_turn(
        self,
        prompt_tokens: int,
        completion_tokens: int,
        thoughts_tokens: int = 0,
    ) -> dict[str, Any]:
        """Registra un turno de conversación y actualiza los costos acumulados."""
        self.total_turns += 1
        effective_output_tokens = completion_tokens + thoughts_tokens

        in_rate, out_rate = get_model_rates(self.model_name, self.is_local)

        turn_cost = (prompt_tokens * in_rate) + (effective_output_tokens * out_rate)
        turn_total_tokens = prompt_tokens + effective_output_tokens

        self.last_prompt_tokens = prompt_tokens
        self.last_completion_tokens = effective_output_tokens
        self.last_tokens = turn_total_tokens
        self.last_cost_usd = turn_cost

        self.total_prompt_tokens += prompt_tokens
        self.total_completion_tokens += effective_output_tokens
        self.total_tokens += turn_total_tokens
        self.total_cost_usd += turn_cost

        is_budget_exceeded = False
        if self.max_session_cost_usd is not None and self.total_cost_usd >= self.max_session_cost_usd:
            if not self.budget_warned:
                self.budget_warned = True
                is_budget_exceeded = True

        return {
            "turn_cost_usd": turn_cost,
            "total_cost_usd": self.total_cost_usd,
            "turn_tokens": turn_total_tokens,
            "total_tokens": self.total_tokens,
            "budget_exceeded": is_budget_exceeded,
        }

    def format_snapbar(self) -> str:
        """Genera el banner visual compacto (Snapbar) de consumo de tokens y costo."""
        sep = "─" * 80

        if self.is_local:
            cost_str = "100% GRATIS (Local / Offline)"
            turn_cost_str = "$0.00"
        else:
            cost_str = f"~${self.total_cost_usd:.4f} USD"
            turn_cost_str = f"+${self.last_cost_usd:.4f}"

        turn_info = f"+{self.last_prompt_tokens:,} in / +{self.last_completion_tokens:,} out ({turn_cost_str})"
        session_info = f"{self.total_tokens:,} tokens ({cost_str})"
        model_info = f"{self.backend_name.upper()} ({self.model_name})"

        snapbar = (
            f"\n{sep}\n"
            f"📊 [SNAPBAR DE CONSUMO] Turno: {turn_info} | Sesión: {session_info} | {model_info}\n"
            f"{sep}"
        )

        if self.max_session_cost_usd is not None and self.total_cost_usd >= self.max_session_cost_usd:
            alert = (
                f"\n⚠️  [ALERTA DE PRESUPUESTO] Has alcanzado tu límite de ${self.max_session_cost_usd:.2f} USD "
                f"(Gasto acumulado: ${self.total_cost_usd:.4f} USD).\n"
            )
            snapbar = alert + snapbar

        return snapbar

    def get_summary(self) -> dict[str, Any]:
        """Obtiene un resumen estructurado para tools de IA."""
        if self.is_local:
            cost_info = "100% GRATIS (Motor Local / Offline)"
        else:
            cost_info = f"~${self.total_cost_usd:.4f} USD"

        return {
            "backend": self.backend_name,
            "model": self.model_name,
            "is_local": self.is_local,
            "turns": self.total_turns,
            "total_tokens": self.total_tokens,
            "prompt_tokens": self.total_prompt_tokens,
            "completion_tokens": self.total_completion_tokens,
            "total_cost_usd": round(self.total_cost_usd, 6),
            "cost_formatted": cost_info,
        }
