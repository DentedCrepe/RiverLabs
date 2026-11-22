# syntax=docker/dockerfile:1
# Build context: repository root.

# --- Stage 1: build the Rust engine wheel and export locked backend dependencies ---
FROM rust:1.97-slim-trixie AS builder
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 python3-venv python3-pip \
 && rm -rf /var/lib/apt/lists/*
RUN python3 -m venv /opt/build && /opt/build/bin/pip install --no-cache-dir "maturin==1.16.0" "uv==0.11.32"
WORKDIR /src
COPY engine/ engine/
RUN /opt/build/bin/maturin build --release --manifest-path engine/riverlab-py/Cargo.toml --out /wheels
COPY backend/pyproject.toml backend/uv.lock backend/
RUN cd backend && /opt/build/bin/uv export --frozen --no-dev --no-emit-project \
      --no-emit-package riverlab-engine --no-hashes -o /wheels/requirements.txt

# --- Stage 2: runtime ---
FROM python:3.13-slim-trixie AS runtime
ENV PYTHONDONTWRITEBYTECODE=1 PYTHONUNBUFFERED=1 PIP_NO_CACHE_DIR=1
WORKDIR /app
COPY --from=builder /wheels /wheels
RUN pip install -r /wheels/requirements.txt && pip install /wheels/riverlab_engine-*.whl
COPY backend/pyproject.toml backend/alembic.ini ./
COPY backend/alembic/ alembic/
COPY backend/src/ src/
RUN pip install --no-deps . && rm -rf /wheels
RUN useradd --system --uid 10001 riverlab
USER riverlab
EXPOSE 8000
# Development convenience: apply migrations, then serve. Production deployments should run
# migrations as a separate, reviewed step.
CMD ["sh", "-c", "alembic upgrade head && exec uvicorn riverlab_api.main:app --host 0.0.0.0 --port 8000"]
