FROM rust:alpine AS builder

RUN \
	mkdir -p /da && \
	mkdir -p /da/src && \
	mkdir -p /da/state && \
	mkdir -p /da/keystore && \
	mkdir -p /da/bin
	
WORKDIR /da/src
COPY . /da/src

RUN apk add --update gcc make g++ zlib-dev alpine-sdk llvm-dev clang-dev build-base
RUN apk add --no-cache libpcap-dev
RUN apk add --no-cache -U musl-dev
RUN apk add libpcap gettext

RUN cargo build --release

RUN \ 
	mv /da/src/target/release/libp2p_test_tool /da/bin && \
	chmod +x /da/bin/libp2p_test_tool && cp /da/src/entrypoint.sh /da && chmod +x /da/entrypoint.sh && \
	rm -rf /da/src 

FROM rust:alpine

RUN apk add libpcap gettext

COPY --from=builder /da/ /da

WORKDIR /da

ENTRYPOINT [ "./entrypoint.sh" ]