"""Hera AI Agent — Modern DJ Web Interface (Streamlit)."""

import asyncio
from pathlib import Path
import sys

import streamlit as st

# Cargar variables de entorno
try:
    from dotenv import load_dotenv
    load_dotenv()
except Exception:
    pass

from hera.agent.backends import BACKENDS
from hera.agent.brain import HeraBrain
from hera.domain.community import CommunityStats
from hera.domain.config import AgentConfig, HeraConfig

# ─── Configuración de la página ──────────────────────────────────────────────
st.set_page_config(
    page_title="Hera DJ Agent",
    page_icon="🎧",
    layout="wide",
    initial_sidebar_state="expanded",
)

# ─── Estilos CSS — Dark Mode & Acentos Neón para DJs ────────────────────────
st.markdown(
    """
<style>
    /* Estilos globales */
    .stApp {
        background-color: #0b0e14;
        color: #e0e6ed;
    }
    
    /* Header principal */
    .hera-header {
        display: flex;
        align-items: center;
        gap: 15px;
        padding: 10px 0 20px 0;
        border-bottom: 1px solid #1e2638;
        margin-bottom: 20px;
    }
    .hera-title {
        font-size: 2.2rem;
        font-weight: 800;
        background: linear-gradient(90deg, #00f2fe 0%, #4facfe 100%);
        -webkit-background-clip: text;
        -webkit-text-fill-color: transparent;
        margin: 0;
    }
    .hera-subtitle {
        font-size: 0.95rem;
        color: #8892b0;
        margin: 0;
    }
    
    /* Tarjetas de métricas en Sidebar */
    .metric-card {
        background: #141b2d;
        border: 1px solid #1f2a44;
        border-radius: 10px;
        padding: 12px 16px;
        margin-bottom: 12px;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    }
    .metric-title {
        font-size: 0.8rem;
        text-transform: uppercase;
        letter-spacing: 1px;
        color: #64ffda;
        font-weight: 700;
        margin-bottom: 6px;
    }
    .metric-value {
        font-size: 1.3rem;
        font-weight: 700;
        color: #ffffff;
    }
    .metric-subtitle {
        font-size: 0.78rem;
        color: #8892b0;
    }
    
    /* Badge de estado P2P */
    .status-badge-online {
        display: inline-block;
        background-color: rgba(0, 230, 118, 0.15);
        color: #00e676;
        border: 1px solid #00e676;
        padding: 3px 10px;
        border-radius: 20px;
        font-size: 0.75rem;
        font-weight: 700;
    }
    .status-badge-offline {
        display: inline-block;
        background-color: rgba(255, 171, 0, 0.15);
        color: #ffab00;
        border: 1px solid #ffab00;
        padding: 3px 10px;
        border-radius: 20px;
        font-size: 0.75rem;
        font-weight: 700;
    }
    
    /* Botones rápidos */
    .stButton>button {
        background: #18223a;
        color: #ccd6f6;
        border: 1px solid #233554;
        border-radius: 8px;
        transition: all 0.2s ease;
        width: 100%;
        text-align: left;
    }
    .stButton>button:hover {
        background: #233554;
        color: #64ffda;
        border-color: #64ffda;
    }
</style>
""",
    unsafe_allow_html=True,
)


# ─── Funciones Auxiliares de Inicialización ──────────────────────────────────
@st.cache_resource
def get_hera_config() -> HeraConfig:
    cfg_path = Path("config/hera.toml")
    if cfg_path.exists():
        return HeraConfig.load(cfg_path).resolve_paths(Path("."))
    return HeraConfig().resolve_paths(Path("."))


def get_community_info():
    """Consulta rápida y no bloqueante de las métricas de Soulseek."""
    cfg = get_hera_config()
    stats = CommunityStats(base_url=cfg.providers.slskd_url or "http://localhost:5030")
    try:
        loop = asyncio.new_event_loop()
        res = loop.run_until_complete(
            stats.get_sharing_summary(cfg.library_dir, Path(cfg.data_dir) / "sets")
        )
        loop.close()
        return res
    except Exception:
        return {
            "is_live": False,
            "tracks_shared": 0,
            "total_size_gb": 0.0,
            "uploads_count": 0,
            "unique_peers_served": 0,
        }


# ─── Inicialización de Estado de Sesión ──────────────────────────────────────
if "messages" not in st.session_state:
    st.session_state.messages = [
        {
            "role": "assistant",
            "content": (
                "👋 ¡Hola DJ! Soy **Hera**, tu superagente de música.\n\n"
                "Puedo ayudarte a:\n"
                "* 🔍 Buscar y descargar tracks lossless de Soulseek.\n"
                "* 🎛️ Calcular transiciones armónicas con la rueda de Camelot.\n"
                "* 🎧 Armar y curar sets completos organizados por BPM y tono.\n"
                "* 📦 Organizar y verificar la calidad de tu biblioteca.\n\n"
                "¿Qué quieres tocar o buscar hoy?"
            ),
        }
    ]

if "brain" not in st.session_state:
    cfg = get_hera_config()
    brain = HeraBrain(cfg.agent)
    # Inicializar el agente en background
    loop = asyncio.new_event_loop()
    loop.run_until_complete(brain.initialize())
    loop.close()
    st.session_state.brain = brain


# ─── SIDEBAR: Panel de Control del DJ ───────────────────────────────────────
with st.sidebar:
    st.markdown("### 🎧 Hera Control Hub")
    
    # 1. Selector de Backend de IA
    backend_options = ["auto", "vertex", "gemini", "ollama", "openai", "anthropic", "lmstudio"]
    current_backend = st.session_state.brain.config.backend
    idx = backend_options.index(current_backend) if current_backend in backend_options else 0
    
    selected_backend = st.selectbox(
        "🧠 Motor de Inteligencia:",
        options=backend_options,
        index=idx,
        help="Selecciona el proveedor de IA. 'ollama' y 'lmstudio' son 100% locales y gratuitos.",
    )
    
    if selected_backend != current_backend:
        st.session_state.brain.config.backend = selected_backend
        loop = asyncio.new_event_loop()
        loop.run_until_complete(st.session_state.brain.initialize())
        loop.close()
        st.rerun()

    # 2. Métricas de Red Soulseek P2P
    comm_stats = get_community_info()
    is_live = comm_stats.get("is_live", False)
    
    st.markdown("---")
    st.markdown("#### 🌐 Soulseek P2P (Buen Ciudadano)")
    
    status_html = (
        '<span class="status-badge-online">🟢 EN LÍNEA / COMPARTIENDO</span>'
        if is_live
        else '<span class="status-badge-offline">🟡 LOCAL (slskd en pausa)</span>'
    )
    st.markdown(status_html, unsafe_allow_html=True)
    
    st.markdown(
        f"""
        <div class="metric-card" style="margin-top: 10px;">
            <div class="metric-title">Biblioteca Compartida</div>
            <div class="metric-value">{comm_stats.get('tracks_shared', 0)} tracks</div>
            <div class="metric-subtitle">{comm_stats.get('total_size_gb', 0.0):.2f} GB curados y lossless</div>
        </div>
        """,
        unsafe_allow_html=True,
    )
    
    # 3. Snapbar de Costos & Tokens
    st.markdown("---")
    st.markdown("#### 📊 Consumo de Tokens")
    
    tracker = getattr(st.session_state.brain, "cost_tracker", None)
    if tracker:
        summary = tracker.get_summary()
        tokens_val = summary["total_tokens"]
        cost_val = summary["cost_formatted"]
        turns_val = summary["turns"]
    else:
        tokens_val = 0
        cost_val = "$0.00 USD"
        turns_val = 0

    st.markdown(
        f"""
        <div class="metric-card">
            <div class="metric-title">Gasto de Sesión</div>
            <div class="metric-value">{cost_val}</div>
            <div class="metric-subtitle">{tokens_val:,} tokens en {turns_val} turnos</div>
        </div>
        """,
        unsafe_allow_html=True,
    )

    # 4. Acciones Rápidas para el DJ
    st.markdown("---")
    st.markdown("#### ⚡ Acciones Rápidas")
    
    quick_prompt = None
    if st.button("🎛️ Transiciones Camelot (8A @ 124 BPM)"):
        quick_prompt = "Recomiéndame transiciones armónicas compatibles para un track en 8A a 124 BPM"
    if st.button("📦 Ver Inventario de Biblioteca"):
        quick_prompt = "Muéstrame el inventario y estado actual de mi biblioteca de música"
    if st.button("🤝 Ver Impacto en Soulseek"):
        quick_prompt = "¿Cómo va mi colaboración y estadísticas en la red Soulseek?"
    if st.button("💰 Consultar Consumo de Tokens"):
        quick_prompt = "¿Cuánto llevamos gastado en tokens en esta sesión?"
        
    st.markdown("---")
    if st.button("🗑️ Limpiar Historial"):
        st.session_state.messages = [st.session_state.messages[0]]
        st.rerun()


# ─── ÁREA PRINCIPAL: Header & Chat Interactivo ──────────────────────────────
st.markdown(
    """
    <div class="hera-header">
        <div>
            <h1 class="hera-title">🎧 HERA AI AGENT</h1>
            <p class="hera-subtitle">Consola de Curaduría Musical, Mezcla Armónica y P2P para DJs</p>
        </div>
    </div>
    """,
    unsafe_allow_html=True,
)

# Renderizar historial de mensajes
for msg in st.session_state.messages:
    role = msg["role"]
    avatar = "🎧" if role == "assistant" else "👤"
    with st.chat_message(role, avatar=avatar):
        st.markdown(msg["content"])

# Capturar input del usuario (desde chat_input o desde Quick Actions)
user_prompt = st.chat_input("Pregúntale a Hera algo sobre música, playlists o transiciones...")
if quick_prompt:
    user_prompt = quick_prompt

if user_prompt:
    # 1. Agregar mensaje del usuario
    st.session_state.messages.append({"role": "user", "content": user_prompt})
    with st.chat_message("user", avatar="👤"):
        st.markdown(user_prompt)

    # 2. Generar respuesta con HeraBrain
    with st.chat_message("assistant", avatar="🎧"):
        response_placeholder = st.empty()
        full_response_chunks = []

        def on_stream_token(token: str):
            full_response_chunks.append(token)
            response_placeholder.markdown("".join(full_response_chunks) + " ▌")


        with st.spinner("Hera está pensando y ejecutando tools..."):
            try:
                loop = asyncio.new_event_loop()
                ans = loop.run_until_complete(
                    st.session_state.brain.chat(
                        user_prompt,
                        on_token=on_stream_token,
                        print_to_stdout=False,
                    )
                )
                loop.close()
                response_placeholder.markdown(ans)
                st.session_state.messages.append({"role": "assistant", "content": ans})
                st.rerun()
            except Exception as e:
                err = f"⚠️ Error comunicando con el agente: {e}"
                response_placeholder.markdown(err)
                st.session_state.messages.append({"role": "assistant", "content": err})
