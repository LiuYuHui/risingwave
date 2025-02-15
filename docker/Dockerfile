FROM ubuntu:22.04 as builder

ENV LANG en_US.utf8

RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -y install make build-essential cmake protobuf-compiler curl pkg-config bash lld maven

SHELL ["/bin/bash", "-c"]

RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- --no-modify-path --default-toolchain none -y

RUN mkdir -p /risingwave

WORKDIR /risingwave

COPY ./ /risingwave

ENV PATH /root/.cargo/bin/:$PATH

ENV IN_CONTAINER=1

ARG GIT_SHA
ENV GIT_SHA=$GIT_SHA

# We need to add the `rustfmt` dependency, otherwise `risingwave_pb` will not compile
RUN rustup self update \
  && rustup set profile minimal \
  && rustup default $(cat "/risingwave/rust-toolchain") \
  && rustup component add rustfmt

RUN cargo fetch
RUN mkdir -p /risingwave/bin/connector-node

RUN cargo build -p risingwave_cmd -p risingwave_cmd_all --release --features "static-link static-log-level" && \
  mv /risingwave/target/release/{frontend,compute-node,meta-node,compactor,risingwave} /risingwave/bin/ && \
  cargo clean
RUN for component in "risingwave" "compute-node" "meta-node" "frontend" "compactor"; do \
  objcopy --compress-debug-sections=zlib-gnu /risingwave/bin/${component}; \
  done

RUN cd risingwave-connector-node && mvn -B package -Dmaven.test.skip=true
RUN tar -zxvf /risingwave/risingwave-connector-node/assembly/target/risingwave-connector-1.0.0.tar.gz -C /risingwave/bin/connector-node

FROM ubuntu:22.04 as image-base
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get -y install ca-certificates openjdk-11-jdk && rm -rf /var/lib/{apt,dpkg,cache,log}/

FROM image-base as risingwave
LABEL org.opencontainers.image.source https://github.com/risingwavelabs/risingwave
RUN mkdir -p /risingwave/bin/connector-node
COPY --from=builder /risingwave/bin/risingwave /risingwave/bin/risingwave
COPY --from=builder /risingwave/bin/connector-node /risingwave/bin/connector-node
# Set default playground mode to docker-playground profile
ENV PLAYGROUND_PROFILE docker-playground
ENTRYPOINT [ "/risingwave/bin/risingwave" ]
CMD [ "playground" ]
