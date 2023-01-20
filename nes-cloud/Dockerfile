FROM rustlang/rust:nightly as builder

RUN apt-get update && apt-get install -y libsixel-dev build-essential tree
WORKDIR /potatis

# mos6502
RUN cargo new --lib common
COPY common/Cargo.toml common/Cargo.toml

# mos6502
RUN cargo new --lib mos6502
COPY mos6502/Cargo.toml mos6502/Cargo.toml

# nes
RUN cargo new --lib nes
COPY nes/Cargo.toml nes/Cargo.toml

# cloud
RUN mkdir nes-cloud
COPY nes-cloud/Cargo.toml nes-cloud/Cargo.toml

WORKDIR /potatis/nes-cloud
RUN mkdir src src/shared src/server src/instance && \
  touch src/shared/lib.rs && \
  echo 'fn main() {}' > src/server/main.rs && \
  echo 'fn main() {}' > src/instance/main.rs

# build & cache deps
WORKDIR /potatis
COPY nes-cloud/Cargo.toml.docker /potatis/Cargo.toml
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

# copy real src
COPY common /potatis/common
COPY mos6502 /potatis/mos6502
COPY nes /potatis/nes
COPY nes-cloud /potatis/nes-cloud

# build
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/potatis/target \
 cargo build --release && \
 mv target/release/nes-cloud-app /root && \
 mv target/release/nes-cloud-instance /root

# real img
FROM debian:bullseye
RUN apt-get update && apt-get install -y libsixel-bin htop

COPY --from=builder /root/nes-cloud-app /
COPY --from=builder /root/nes-cloud-instance /
COPY nes-cloud/resources.yaml /
COPY nes-cloud/included-roms /included-roms

EXPOSE 4444/tcp
EXPOSE 5555/tcp
EXPOSE 6666/tcp
EXPOSE 7777/tcp

CMD ["./nes-cloud-app", "--log-to-file", "--instance-bin", "./nes-cloud-instance"]