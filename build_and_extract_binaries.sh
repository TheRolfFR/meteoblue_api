set -x

pwd
ls -al

podman build -t meteoblue_api:dev . --format docker

podman rm extract_meteoblue 2>/dev/null || true

mkdir -p target/
podman create --name extract_meteoblue meteoblue_api:dev
podman cp extract_meteoblue:/root/meteoblue_api target/meteoblue_api_linux_musl
podman cp extract_meteoblue:/root/meteoblue_bin target/meteoblue_bin_linux_musl
podman cp extract_meteoblue:/root/meteoblue_graph target/meteoblue_graph_linux_musl

chmod +x ./target/meteoblue_graph_linux_musl
./target/meteoblue_graph_linux_musl --help
