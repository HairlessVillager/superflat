podman-compose up -d
podman logs mc-chunky-gen -f
podman exec mc-chunky-gen rcon-cli chunky center 16384 16384
podman exec mc-chunky-gen rcon-cli chunky radius 100
podman exec mc-chunky-gen rcon-cli chunky start
podman exec mc-chunky-gen rcon-cli save-all flush
podman cp mc-chunky-gen:/data/world/region/r.32.32.mca .
podman-compose down
