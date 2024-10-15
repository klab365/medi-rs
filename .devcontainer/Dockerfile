FROM mcr.microsoft.com/devcontainers/rust:1

# Pass ID to the container
ARG UID
RUN usermod -u $UID vscode

USER vscode

# Install rust version and components
RUN rustup default 1.80
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup component add rustfmt
RUN rustup component add clippy
RUN rustup component add llvm-tools-preview

# Install cago-llvm-cov for code coverage
RUN curl \
    --proto '=https' \
    --tlsv1.2 -fsSL \
    https://github.com/taiki-e/cargo-llvm-cov/releases/latest/download/cargo-llvm-cov-$(rustc -vV | grep '^host:' | cut -d' ' -f2).tar.gz \
    | tar xzf - -C "/usr/local/cargo/bin"
