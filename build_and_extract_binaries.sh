podman build -t meteoblue_api:dev . --format docker
podman rm extract_meteoblue 2>/dev/null || true
podman create --name extract_meteoblue meteoblue_api:dev
podman cp extract_meteoblue:/root/meteoblue_api ./meteoblue_api_linux_musl
podman cp extract_meteoblue:/root/meteoblue_bin ./meteoblue_bin_linux_musl
podman cp extract_meteoblue:/root/meteoblue_graph ./meteoblue_graph_linux_musl
./meteoblue_graph_linux_musl --help
