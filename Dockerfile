# Builds the web app and the server into one image.
#
# Each platform is built on a runner of its own architecture (see .github/workflows/release.yml),
# so the build targets the architecture it runs on.

FROM docker.io/clux/muslrust:stable AS builder
ARG TARGETARCH
# release = the production profile; pass --build-arg PROFILE=ci for a faster build without LTO.
ARG PROFILE=release
USER root
WORKDIR /app

# dx (the Dioxus CLI) builds the web app. It is downloaded only when the cache mount is empty and then
# reused by later builds. The cache is per architecture, because the binary is.
COPY scripts/install-dx.sh .
RUN --mount=type=cache,id=t-rex-dx-${TARGETARCH},target=/opt/dx-cache \
    if [ ! -x /opt/dx-cache/bin/dx ]; then DX_INSTALL=/opt/dx-cache bash install-dx.sh; fi \
 && cp /opt/dx-cache/bin/dx "${CARGO_HOME:-$HOME/.cargo}/bin/dx"

# The Rust target triple for the platform this stage runs for.
RUN case "$TARGETARCH" in \
      amd64) echo x86_64-unknown-linux-musl ;; \
      arm64) echo aarch64-unknown-linux-musl ;; \
      *) echo "unsupported platform: $TARGETARCH" >&2; exit 1 ;; \
    esac > /rust-target

COPY . .
# The image sets CARGO_HOME=/opt/cargo, so the registry and git caches are mounted there. Mounting
# another path leaves the downloads in the image layer, which makes the commit of the layer slow.
RUN --mount=type=cache,id=t-rex-cargo-registry,target=/opt/cargo/registry \
    --mount=type=cache,id=t-rex-cargo-git,target=/opt/cargo/git \
    --mount=type=cache,id=t-rex-target-${TARGETARCH}-${PROFILE},target=/app/target \
    RUSTFLAGS='-C target-feature=+crt-static' \
    cargo build --profile ${PROFILE} --target "$(cat /rust-target)" --bin server \
 && cp "target/$(cat /rust-target)/${PROFILE}/server" /server

# Runtime: distroless static, the server is a static binary. It has CA certificates and a non-root
# user, but no shell or package manager. Pinned by digest so updates are proposed by Dependabot.
FROM gcr.io/distroless/static-debian13:nonroot@sha256:aafe993665bf6200db38a02c11a9351e782a84b6a45e7f33b6695638c88e3931 AS runtime
LABEL org.opencontainers.image.source="https://github.com/aellwein/t-rex"
LABEL org.opencontainers.image.licenses="MIT"
COPY --from=builder /server /usr/local/bin/server
# Numeric, so Kubernetes can check runAsNonRoot. 65532 is the nonroot user of distroless.
USER 65532:65532
EXPOSE 8080
CMD ["/usr/local/bin/server"]
