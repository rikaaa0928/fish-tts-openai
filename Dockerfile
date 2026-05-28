# Build stage
FROM rust:1.95-alpine AS builder

WORKDIR /usr/src/app

# Install build dependencies
RUN apk add --no-cache musl-dev

COPY Cargo.toml Cargo.lock ./
# Create dummy src file to build dependencies and cache them
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN <<EOF_SCRIPT
# 1. 创建目录（如果环境变量未定义，默认会使用 $HOME/.cargo）
mkdir -vp \${CARGO_HOME:-\$HOME/.cargo}

# 2. 写入镜像源配置
cat << EOF | tee -a \${CARGO_HOME:-\$HOME/.cargo}/config.toml
[source.crates-io]
replace-with = 'mirror'

[source.mirror]
registry = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"

[registries.mirror]
index = "sparse+https://mirrors.tuna.tsinghua.edu.cn/crates.io-index/"
EOF

EOF_SCRIPT
RUN cargo build --release
RUN rm -f target/release/deps/fish_openai_proxy*

# Copy actual source code
COPY src ./src
RUN cargo build --release

# Final stage
FROM alpine:latest

# Install runtime dependencies (ca-certificates for HTTPs requests)
RUN apk add --no-cache ca-certificates

WORKDIR /app

# Copy the binary from the builder stage
COPY --from=builder /usr/src/app/target/release/fish_openai_proxy .

# Expose port
EXPOSE 8000

# Set executable
CMD ["./fish_openai_proxy"]
