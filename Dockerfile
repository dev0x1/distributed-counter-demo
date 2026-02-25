# syntax=docker/dockerfile:1.7

ARG DEBIAN_VERSION=bookworm
ARG CMAKE_VERSION=4.2.3

############################
# Builder
############################
FROM --platform=$TARGETPLATFORM debian:${DEBIAN_VERSION}-slim AS builder

ARG DEBIAN_FRONTEND=noninteractive
ARG CMAKE_VERSION
ARG TARGETARCH

# Rust locations
ENV RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/cargo/bin:/usr/local/bin:/usr/bin:/bin

# Build deps
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates curl git xz-utils \
    build-essential pkg-config \
    ninja-build \
    python3 \
    bash \
  && rm -rf /var/lib/apt/lists/*

# Install prebuilt CMake (Kitware)
RUN case "${TARGETARCH}" in \
      "arm64")  CMAKE_ARCH="aarch64" ;; \
      "amd64")  CMAKE_ARCH="x86_64" ;; \
      *) echo "Unsupported TARGETARCH: ${TARGETARCH}" && exit 1 ;; \
    esac && \
    curl -fsSL -o /tmp/cmake.tar.gz \
      "https://github.com/Kitware/CMake/releases/download/v${CMAKE_VERSION}/cmake-${CMAKE_VERSION}-linux-${CMAKE_ARCH}.tar.gz" && \
    tar -xzf /tmp/cmake.tar.gz -C /opt && \
    ln -sf /opt/cmake-${CMAKE_VERSION}-linux-${CMAKE_ARCH}/bin/* /usr/local/bin/ && \
    rm -f /tmp/cmake.tar.gz && \
    cmake --version

# Install Rust (stable)
RUN curl -fsSL https://sh.rustup.rs | sh -s -- -y --profile minimal && \
    rustup toolchain install stable && \
    rustup default stable && \
    rustc --version && cargo --version

WORKDIR /src

# Clone upstream during build, needed until package is available in crates.io registry
RUN git clone https://github.com/tashigg/tashi-vertex-rs.git upstream

# Copy demo + scripts into the build context
COPY demo /src/demo
COPY scripts /src/scripts
RUN chmod +x /src/scripts/*.sh

WORKDIR /src/upstream
RUN --mount=type=cache,target=/opt/cargo/registry \
    --mount=type=cache,target=/opt/cargo/git \
    cargo build --release --example pingback

# Build demo (this compiles upstream too via the path dependency)
WORKDIR /src/demo
RUN --mount=type=cache,target=/opt/cargo/registry \
    --mount=type=cache,target=/opt/cargo/git \
    cargo build --release

# RUN find /src -name "libtashi-vertex.so*" -maxdepth 6 -print

############################
# Runtime
############################
FROM --platform=$TARGETPLATFORM debian:${DEBIAN_VERSION}-slim AS runtime

ARG DEBIAN_FRONTEND=noninteractive

# Minimal runtime deps (bash for entrypoint scripts; ca-certs for TLS if anything ever needs it)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    bash \
    file \
    libc-bin \
  && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy only the built binaries + scripts
COPY --from=builder /src/demo/target/release/tashi-demo-node /usr/local/bin/tashi-demo-node
COPY --from=builder /src/demo/target/release/tashi-demo-keygen /usr/local/bin/tashi-demo-keygen
COPY --from=builder /src/scripts /app/scripts
COPY --from=builder /src/upstream/target/release/lib/libtashi-vertex.so /usr/local/lib/libtashi-vertex.so
ENV LD_LIBRARY_PATH=/usr/local/lib
RUN ldconfig

RUN chmod +x /app/scripts/*.sh

CMD ["bash"]
